//! Shell integration, as legacy does it: a script typed into the shell
//! hooks its prompt and reports commands with `OSC 4545` sequences —
//! `A` (installing), `B` (ready), `ShellPromptBegins` / `ShellPromptEnds`,
//! `CommandStarted;<base64>`, `CommandExited;<code>`, `SetCwd;<base64>`.
//! bash runs the legacy script; zsh and fish run equivalents that send the
//! same events.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

const BASH: &str = include_str!("shell_integration/bash.sh");
const ZSH: &str = include_str!("shell_integration/zsh.zsh");
const FISH: &str = include_str!("shell_integration/fish.fish");

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShellEvent {
    Installing,
    Ready,
    PromptBegins,
    PromptEnds,
    CommandStarted(String),
    CommandExited(i32),
    Cwd(String),
}

const PREFIX: &[u8] = b"\x1b]4545;";
/// Longest event kept while waiting for its end (a command can be long).
const MAX_EVENT: usize = 64 * 1024;

/// Takes the `OSC 4545` events out of shell output, also when one is split
/// between reads; everything else passes through unchanged.
#[derive(Default)]
pub struct Scanner {
    pending: Vec<u8>,
}

fn decode(text: &str) -> String {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    STANDARD.decode(compact).ok().map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default()
}

fn event(body: &[u8]) -> Option<ShellEvent> {
    let body = String::from_utf8_lossy(body);
    let (kind, value) = body.split_once(';').unwrap_or((&body, ""));
    Some(match kind {
        "A" => ShellEvent::Installing,
        "B" => ShellEvent::Ready,
        "ShellPromptBegins" => ShellEvent::PromptBegins,
        "ShellPromptEnds" => ShellEvent::PromptEnds,
        "CommandStarted" => ShellEvent::CommandStarted(decode(value).trim_end_matches('\n').to_owned()),
        "CommandExited" => ShellEvent::CommandExited(value.trim().parse().unwrap_or(0)),
        "SetCwd" => ShellEvent::Cwd(decode(value)),
        _ => return None,
    })
}

impl Scanner {
    pub fn scan(&mut self, bytes: &[u8]) -> (Vec<u8>, Vec<ShellEvent>) {
        let mut input = std::mem::take(&mut self.pending);
        input.extend_from_slice(bytes);
        let (mut out, mut events) = (Vec::with_capacity(input.len()), Vec::new());
        let mut i = 0;
        while i < input.len() {
            if input[i] != 0x1b {
                out.push(input[i]);
                i += 1;
                continue;
            }
            let rest = &input[i..];
            // A prefix cut off at the end of this read: wait for the next one.
            if rest.len() < PREFIX.len() && PREFIX.starts_with(rest) {
                self.pending = rest.to_vec();
                break;
            }
            if !rest.starts_with(PREFIX) {
                out.push(input[i]);
                i += 1;
                continue;
            }
            let body = &rest[PREFIX.len()..];
            let end = body.iter().enumerate().find_map(|(j, b)| match b {
                0x07 => Some((j, 1)),
                0x1b if body.get(j + 1) == Some(&b'\\') => Some((j, 2)),
                _ => None,
            });
            match end {
                Some((j, terminator)) => {
                    events.extend(event(&body[..j]));
                    i += PREFIX.len() + j + terminator;
                }
                None if rest.len() < MAX_EVENT => {
                    self.pending = rest.to_vec();
                    break;
                }
                None => {
                    // Not an event after all: let the terminal have it.
                    out.extend_from_slice(rest);
                    break;
                }
            }
        }
        (out, events)
    }
}

/// The line typed into a fresh shell to install the integration: each shell
/// picks its own script; any shell then leaves the alternate screen the
/// terminal switched to while the line runs, and clears the old prompt
/// line so the next prompt takes its place. A leading space keeps it out
/// of fish and `ignorespace` histories; bash also deletes it from history.
pub fn install_line() -> String {
    let bash = format!("history -d $(history 1 | awk '{{print $1}}') 2>/dev/null\n{BASH}");
    let part = |var: &str, script: &str| format!("test -n \"${var}\" && eval \"$(printf '%s' {} | base64 -d | gunzip)\"", packed(script));
    format!(
        " {}; {}; {}; printf '\\033[?1049l\\r\\033[K'\r",
        part("FISH_VERSION", FISH),
        part("ZSH_VERSION", ZSH),
        part("BASH_VERSION", &bash)
    )
}

/// A script as legacy sends it (no comment lines, no indentation), gzipped
/// and base64-encoded so the typed line stays short: line editors such as
/// bash 3.2's readline echo every character.
fn packed(script: &str) -> String {
    use std::io::Write;
    let lines: Vec<&str> = script.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
    let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::best());
    let _ = gz.write_all(lines.join("\n").as_bytes());
    let _ = gz.write_all(b"\n");
    STANDARD.encode(gz.finish().unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn events_are_taken_out_even_when_split() {
        let mut scanner = Scanner::default();
        let started = format!("\x1b]4545;CommandStarted;{}\x07", STANDARD.encode("ls -la"));
        let stream = format!("before{started}mid\x1b]4545;CommandExited;2\x1b\\after\x1b[0m");
        let bytes = stream.as_bytes();
        let (mut out, mut events) = (Vec::new(), Vec::new());
        for chunk in bytes.chunks(5) {
            let (o, e) = scanner.scan(chunk);
            out.extend(o);
            events.extend(e);
        }
        assert_eq!(String::from_utf8(out).unwrap(), "beforemidafter\x1b[0m");
        assert_eq!(events, [ShellEvent::CommandStarted("ls -la".into()), ShellEvent::CommandExited(2)]);
    }

    #[test]
    fn wrapped_base64_and_other_osc_pass() {
        let mut scanner = Scanner::default();
        let (out, events) = scanner.scan(b"\x1b]0;title\x07\x1b]4545;SetCwd;L1Vz\nZXJz\x07\x1b]4545;B\x07");
        assert_eq!(out, b"\x1b]0;title\x07");
        assert_eq!(events, [ShellEvent::Cwd("/Users".into()), ShellEvent::Ready]);
    }

    /// Runs `shell` in a pty, installs the integration and runs a command.
    fn events_from(shell: &str, args: &[&str]) -> Vec<ShellEvent> {
        let mut command = portable_pty::CommandBuilder::new(shell);
        command.args(args);
        command.env("HOME", std::env::temp_dir());
        let mut session = crate::session::local_command(command, 120, 40).unwrap();
        let mut events = session.take_events().unwrap();
        let start = Instant::now();
        let mut steps: Vec<(u64, Vec<u8>)> = vec![(500, install_line().into_bytes()), (2500, b"echo termian-ok\r".to_vec()), (3500, b"cd / && false\r".to_vec())];
        let mut scanner = Scanner::default();
        let mut terminal = crate::terminal::Terminal::new(120, 40);
        let mut found = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            if steps.first().is_some_and(|(at, _)| start.elapsed() >= Duration::from_millis(*at)) {
                session.write(&steps.remove(0).1);
            }
            #[allow(deprecated)]
            match events.try_next() {
                Ok(Some(crate::session::SessionEvent::Output(bytes))) => {
                    if std::env::var_os("TERMIAN_DEBUG_SHELL").is_some() {
                        eprintln!("{}", String::from_utf8_lossy(&bytes).escape_debug());
                    }
                    let (out, events) = scanner.scan(&bytes);
                    found.extend(events);
                    // fish waits for answers to its terminal queries, as a real terminal gives them.
                    terminal.feed(&out);
                    let replies = terminal.take_replies();
                    if !replies.is_empty() {
                        session.write(&replies);
                    }
                }
                Ok(Some(_)) | Ok(None) => break,
                Err(_) => std::thread::sleep(Duration::from_millis(20)),
            }
            if found.contains(&ShellEvent::Cwd("/".into())) {
                break;
            }
        }
        session.write(b"exit\r");
        found
    }

    fn check(shell: &str, args: &[&str]) {
        if !std::path::Path::new(shell).exists() {
            eprintln!("{shell} is not installed; skipped");
            return;
        }
        let events = events_from(shell, args);
        assert!(events.contains(&ShellEvent::Ready), "{shell}: {events:?}");
        assert!(events.contains(&ShellEvent::CommandStarted("echo termian-ok".into())), "{shell}: {events:?}");
        assert!(events.contains(&ShellEvent::CommandExited(0)), "{shell}: {events:?}");
        assert!(events.contains(&ShellEvent::CommandStarted("cd / && false".into())), "{shell}: {events:?}");
        assert!(events.contains(&ShellEvent::CommandExited(1)), "{shell}: {events:?}");
        assert!(events.contains(&ShellEvent::Cwd("/".into())), "{shell}: {events:?}");
        assert!(events.contains(&ShellEvent::PromptEnds), "{shell}: {events:?}");
    }

    #[test]
    fn the_install_line_is_short() {
        assert!(install_line().len() < 3200, "{}", install_line().len());
    }

    #[test]
    fn bash_reports_commands() {
        check("/bin/bash", &["--norc", "--noprofile", "-i"]);
    }

    #[test]
    fn zsh_reports_commands() {
        check("/bin/zsh", &["-f", "-i"]);
    }

    #[test]
    fn fish_reports_commands() {
        let fish = ["/opt/homebrew/bin/fish", "/usr/local/bin/fish", "/usr/bin/fish"].into_iter().find(|p| std::path::Path::new(p).exists());
        match fish {
            Some(fish) => check(fish, &["--no-config", "-i"]),
            None => eprintln!("fish is not installed; skipped"),
        }
    }
}
