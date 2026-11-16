//! Background shell history extraction, using the legacy bash/zsh/fish
//! history commands. Output never goes through the terminal or session log.

use std::process::Command;
use std::time::Duration;
#[cfg(unix)]
use std::{process::Stdio, time::Instant};

pub(crate) const MAX_OUTPUT: usize = 4 * 1024 * 1024;
pub(crate) const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    pub fn from_path(path: &str) -> Option<Self> {
        match path.trim().rsplit('/').next()? {
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" => Some(Self::Fish),
            _ => None,
        }
    }

    fn args(self) -> &'static [&'static str] {
        match self {
            Self::Bash => &["--noprofile", "--norc", "-c",
                "HISTFILE=/dev/null; HISTSIZE=10000; HISTTIMEFORMAT=' __TERMIAN_HISTORY__ '; history -c; history -r ~/.bash_history || exit; echo __TERMIAN_HISTORY_BEGIN__; history; echo __TERMIAN_HISTORY_END__"],
            Self::Zsh => &["-f", "-c",
                "unset HISTFILE; HISTSIZE=10000; fc -R ~/.zsh_history || exit; echo __TERMIAN_HISTORY_BEGIN__; fc -ln 1; echo __TERMIAN_HISTORY_END__"],
            Self::Fish => &["--no-config", "-c", "set -g fish_history fish; echo __TERMIAN_HISTORY_BEGIN__; history --reverse --max 10000; echo __TERMIAN_HISTORY_END__"],
        }
    }

    pub fn local_command(self, path: &str) -> Command {
        let mut command = Command::new(path);
        command.args(self.args()).env("BASH_ENV", "/dev/null");
        command
    }

    /// Every argument is quoted, including paths obtained from remote `$SHELL`.
    /// Restrict the path so this also works through a fish login shell.
    pub(crate) fn remote_command(self, path: &str) -> Option<String> {
        let path = path.trim();
        if !path.starts_with('/') || !path.bytes().all(|b| b.is_ascii_alphanumeric() || b"/_-.".contains(&b)) {
            return None;
        }
        let quote = |s: &str| format!("'{}'", s.replace('\\', "\\\\").replace('\'', "'\\''"));
        Some(format!("env BASH_ENV=/dev/null {} {}", quote(path), self.args().iter().map(|s| quote(s)).collect::<Vec<_>>().join(" ")))
    }

    /// Legacy output is oldest first. Bash prefixes each numbered entry with
    /// a marker, so shell banners and unmarked lines cannot become commands.
    pub fn parse(self, output: &str) -> Vec<String> {
        let Some((_, output)) = output.split_once("__TERMIAN_HISTORY_BEGIN__\n") else { return Vec::new() };
        if output.starts_with("__TERMIAN_HISTORY_END__\n") { return Vec::new(); }
        let Some((output, _)) = output.split_once("\n__TERMIAN_HISTORY_END__\n") else { return Vec::new() };
        output.lines().filter_map(|line| {
            let line = if self == Self::Bash {
                let (number, command) = line.split_once("__TERMIAN_HISTORY__ ")?;
                number.trim().parse::<usize>().ok()?;
                command
            } else { line };
            let line = line.trim();
            (!line.is_empty()).then(|| line.to_owned())
        }).collect()
    }
}

/// Extracts without evaluating any history entry. Unsupported shells and
/// failures are silent, as in legacy; no history is written back on exit.
pub(crate) fn local(path: &str) -> Option<Vec<String>> {
    let shell = Shell::from_path(path)?;
    Some(shell.parse(&run(shell.local_command(path), TIMEOUT)?))
}

/// Nonblocking reads let the deadline cover descendants holding stdout open,
/// without leaving a reader thread behind. The whole child process group is
/// stopped on every exit path, including output overflow.
#[cfg(unix)]
fn run(mut command: Command, timeout: Duration) -> Option<String> {
    use std::io::Read;
    use std::os::{fd::AsRawFd, unix::process::CommandExt};
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).process_group(0);
    let mut child = command.spawn().ok()?;
    let result = (|| {
        let mut stdout = child.stdout.take()?;
        let fd = stdout.as_raw_fd();
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
        if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0 {
            return None;
        }
        let start = Instant::now();
        let mut output = Vec::new();
        let mut buffer = [0; 8192];
        let mut eof = false;
        loop {
            if start.elapsed() >= timeout { return None; }
            if !eof {
                match stdout.read(&mut buffer) {
                    Ok(0) => eof = true,
                    Ok(n) => {
                        if output.len() + n > MAX_OUTPUT { return None; }
                        output.extend_from_slice(&buffer[..n]);
                        continue;
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                    Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => return None,
                }
            }
            if eof {
                if let Some(status) = child.try_wait().ok()? {
                    return status.success().then(|| String::from_utf8(output).ok()).flatten();
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    })();
    unsafe { libc::kill(-(child.id() as i32), libc::SIGKILL); }
    let _ = child.wait();
    result
}

#[cfg(not(unix))]
fn run(_: Command, _: Duration) -> Option<String> { None }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_only_supported_shells() {
        assert_eq!(Shell::from_path("/bin/bash"), Some(Shell::Bash));
        assert_eq!(Shell::from_path("/usr/local/bin/zsh"), Some(Shell::Zsh));
        assert_eq!(Shell::from_path("/opt/homebrew/bin/fish"), Some(Shell::Fish));
        assert_eq!(Shell::from_path("/bin/sh"), None);
        assert_eq!(Shell::from_path("/bin/notbash"), None);
        assert!(Shell::Bash.remote_command("/bin/bash; touch /tmp/no").is_none());
        assert!(Shell::Bash.remote_command("banner\n/bin/bash").is_none());
        assert!(Shell::Bash.remote_command("bash").is_none());
    }

    #[test]
    fn parses_legacy_history_output_without_banners_or_bash_numbers() {
        assert_eq!(Shell::Bash.parse("__TERMIAN_HISTORY_BEGIN__\nbanner\n  1  __TERMIAN_HISTORY__ ls -la\n  2  __TERMIAN_HISTORY__ echo привет\n__TERMIAN_HISTORY_END__\n"), ["ls -la", "echo привет"]);
        assert_eq!(Shell::Zsh.parse("__TERMIAN_HISTORY_BEGIN__\nls -la\n\necho привет\n__TERMIAN_HISTORY_END__\n"), ["ls -la", "echo привет"]);
        assert_eq!(Shell::Fish.parse("__TERMIAN_HISTORY_BEGIN__\nls -la\necho hello\n__TERMIAN_HISTORY_END__\n"), ["ls -la", "echo hello"]);
        assert_eq!(Shell::Zsh.parse("__TERMIAN_HISTORY_BEGIN__\necho __TERMIAN_HISTORY_END__\n__TERMIAN_HISTORY_END__\n"), ["echo __TERMIAN_HISTORY_END__"]);
        assert!(Shell::Zsh.parse("banner only\n").is_empty());
        assert!(Shell::Fish.parse("__TERMIAN_HISTORY_BEGIN__\ntruncated\n").is_empty());
    }

    #[test]
    fn startup_banners_are_not_imported_as_commands() {
        for shell in [Shell::Bash, Shell::Zsh, Shell::Fish] {
            let line = if shell == Shell::Bash { "1 __TERMIAN_HISTORY__ echo command" } else { "echo command" };
            let output = format!("startup banner\n__TERMIAN_HISTORY_BEGIN__\n{line}\n__TERMIAN_HISTORY_END__\nexit banner\n");
            assert_eq!(shell.parse(&output), ["echo command"]);
        }
    }

    #[cfg(unix)]
    #[test]
    fn real_shells_read_fixture_history_without_changing_it() {
        for (path, shell, file, contents) in [
            ("/bin/bash", Shell::Bash, ".bash_history", "echo first\necho second\n"),
            ("/bin/zsh", Shell::Zsh, ".zsh_history", ": 1700000000:0;echo first\n: 1700000001:0;echo second\n"),
            ("/opt/homebrew/bin/fish", Shell::Fish, ".local/share/fish/fish_history", "- cmd: echo first\n  when: 1700000000\n- cmd: echo second\n  when: 1700000001\n"),
        ] {
            if !std::path::Path::new(path).exists() { continue; }
            let dir = tempfile::tempdir().unwrap();
            let history = dir.path().join(file);
            std::fs::create_dir_all(history.parent().unwrap()).unwrap();
            std::fs::write(&history, contents).unwrap();
            let mut command = shell.local_command(path);
            command.env("HOME", dir.path()).env("XDG_DATA_HOME", dir.path().join(".local/share")).env("XDG_CONFIG_HOME", dir.path().join(".config")).env("ZDOTDIR", dir.path());
            let output = run(command, std::time::Duration::from_secs(5)).unwrap();
            assert_eq!(shell.parse(&output), ["echo first", "echo second"], "{path}");
            for outer in ["/bin/sh", "/opt/homebrew/bin/fish"] {
                if !std::path::Path::new(outer).exists() { continue; }
                let mut command = Command::new(outer);
                if outer.ends_with("fish") { command.arg("--no-config"); }
                command.args(["-c", &shell.remote_command(path).unwrap()]);
                command.env("HOME", dir.path()).env("XDG_DATA_HOME", dir.path().join(".local/share")).env("XDG_CONFIG_HOME", dir.path().join(".config")).env("ZDOTDIR", dir.path()).env("BASH_ENV", "/dev/null");
                let output = run(command, Duration::from_secs(5)).unwrap();
                assert_eq!(shell.parse(&output), ["echo first", "echo second"], "{path} through {outer}");
            }
            assert_eq!(std::fs::read_to_string(history).unwrap(), contents, "{path} history must stay unchanged");
        }
    }

    #[cfg(unix)]
    #[test]
    fn background_reader_times_out_and_bounds_output() {
        let mut command = std::process::Command::new("/bin/sh");
        command.args(["-c", "sleep 10"]);
        let started = std::time::Instant::now();
        assert!(run(command, std::time::Duration::from_millis(100)).is_none());
        assert!(started.elapsed() < std::time::Duration::from_secs(3));
        let mut command = std::process::Command::new("/bin/sh");
        command.args(["-c", "yes x"]);
        assert!(run(command, std::time::Duration::from_secs(5)).is_none());
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "printf ignored; exit 1"]);
        assert!(run(command, Duration::from_secs(5)).is_none());
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "sleep 10 & exit 0"]);
        let started = Instant::now();
        assert!(run(command, Duration::from_millis(100)).is_none());
        assert!(started.elapsed() < Duration::from_secs(3));
    }
}
