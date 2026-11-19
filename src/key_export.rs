//! Key Export, as in the legacy Keychain: log in to a host and run a script
//! that appends the key's public key to `$1/$2` (by default
//! `~/.ssh/authorized_keys`). Exit status 0 is success; otherwise the
//! script's output is the error. Dropping the [`KeyExport`] cancels it.

use std::sync::Arc;

use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::channel::oneshot;
use russh::ChannelMsg;
use tokio::sync::Mutex;

use crate::login::{Answer, LoginEvent, Stage};
use crate::session::{connect_forwarding, EventPrompter, Prompter, SshTarget};
use crate::vault::SshKey;

pub const LOCATION: &str = "~/.ssh";
pub const FILENAME: &str = "authorized_keys";
pub const SCRIPT: &str = "set -e\ncase \"$1\" in /*) dir=$1;; *) dir=./$1;; esac\nif [ ! -d \"$dir\" ]; then mkdir -m 700 -p \"$dir\"; fi\ncase \"$2\" in ''|.|..|*/*) exit 1;; esac\numask 077\nprintf '%s' \"$3\" >> \"$dir/$2\"";

/// The key's OpenSSH public key: stored, or derived from the private key.
pub fn public_key(key: &SshKey) -> Result<String, String> {
    const INVALID: &str = "Unable to export the public key. Please make sure that your key has the correct format.";
    if let Some(public_key) = key.public_key.as_deref().map(str::trim).filter(|k| !k.is_empty()) {
        if public_key.contains(['\r', '\n', '\0']) {
            return Err(INVALID.into());
        }
        let key = russh::keys::PublicKey::from_openssh(public_key).map_err(|_| INVALID)?;
        return key.to_openssh().map_err(|_| INVALID.into());
    }
    let private_key = key.private_key.as_deref().ok_or(INVALID)?;
    let decoded = crate::keys::decode(private_key, key.passphrase.as_deref()).map_err(|_| INVALID)?;
    decoded.public_key().to_openssh().map_err(|_| INVALID.to_owned())
}

pub fn prepare(script: &str, location: &str, filename: &str, label: &str, public_key: &str) -> String {
    let quote = |value: &str| format!("'{}'", value.replace('\'', "'\\''"));
    let location = match location {
        "~" => r#""$HOME""#.into(),
        _ => location.strip_prefix("~/").map_or_else(|| quote(location), |path| format!(r#""$HOME"/{}"#, quote(path))),
    };
    let label: String = label.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    let lines = format!("# {label} Key by Termius\n{}\n", public_key.trim());
    format!("sh -c {} termian-key-export {location} {} {}", quote(script), quote(filename), quote(&lines))
}

#[derive(Debug, PartialEq)]
pub enum ExportEvent {
    Login(LoginEvent),
    /// Finished: `Err` holds the connection error or the script's output.
    Done(Result<(), String>),
}

pub struct KeyExport {
    answers: UnboundedSender<Answer>,
    events: Option<UnboundedReceiver<ExportEvent>>,
    _stop: oneshot::Sender<()>,
}

impl KeyExport {
    pub fn start(target: SshTarget, script: String) -> Self {
        let (answers, answer_rx) = unbounded();
        let (events_tx, events) = unbounded();
        let (stop, stopped) = oneshot::channel::<()>();
        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime,
                Err(error) => {
                    let _ = events_tx.unbounded_send(ExportEvent::Done(Err(error.to_string())));
                    return;
                }
            };
            runtime.block_on(async {
                tokio::select! {
                    result = run(target, script, answer_rx, events_tx.clone()) => {
                        let _ = events_tx.unbounded_send(ExportEvent::Done(result));
                    }
                    _ = stopped => {}
                }
            });
        });
        Self { answers, events: Some(events), _stop: stop }
    }

    pub fn take_events(&mut self) -> Option<UnboundedReceiver<ExportEvent>> {
        self.events.take()
    }

    pub fn answer(&self, answer: Answer) {
        let _ = self.answers.unbounded_send(answer);
    }
}

async fn run(target: SshTarget, script: String, answers: UnboundedReceiver<Answer>, events: UnboundedSender<ExportEvent>) -> Result<(), String> {
    let emit = events.clone();
    let prompter = Arc::new(Mutex::new(EventPrompter {
        emit: move |event| {
            let _ = emit.unbounded_send(ExportEvent::Login(event));
        },
        answers,
        picked: None,
    }));
    let handle = connect_forwarding(&target, prompter.clone(), None).await?;
    prompter.lock().await.event(LoginEvent::Stage(Stage::Ready));
    let mut channel = handle.channel_open_session().await.map_err(|e| e.to_string())?;
    channel.exec(true, script).await.map_err(|e| e.to_string())?;
    let mut output = Vec::new();
    let mut status = None;
    while let Some(message) = channel.wait().await {
        match message {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => output.extend_from_slice(&data),
            ChannelMsg::ExitStatus { exit_status } => status = Some(exit_status),
            ChannelMsg::Close => break,
            _ => {}
        }
    }
    match status {
        Some(0) => Ok(()),
        status => {
            let output = String::from_utf8_lossy(&output).trim().to_owned();
            Err(if output.is_empty() { format!("The export script failed (exit status {}).", status.map_or("unknown".into(), |s| s.to_string())) } else { output })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::login::HostKeyDecision;
    use crate::session::tests::server;
    use futures::StreamExt;
    use std::time::Duration;

    #[test]
    fn prepare_passes_values_as_shell_arguments() {
        let script = prepare(SCRIPT, LOCATION, FILENAME, "work", "ssh-ed25519 AAAA test\n");
        assert!(script.starts_with("sh -c "));
        assert!(script.contains(r#"termian-key-export "$HOME"/'.ssh' 'authorized_keys'"#));
        assert!(script.ends_with("'# work Key by Termius\nssh-ed25519 AAAA test\n'"));
    }

    #[test]
    fn public_key_is_derived_when_not_stored() {
        let generated = crate::keys::generate(crate::keys::KeyAlgorithm::Ed25519, "").unwrap();
        let mut key = SshKey { id: 1, label: "k".into(), kind: "ED25519".into(), private_key: Some(generated.private_key), public_key: None, passphrase: None, certificate: None };
        assert_eq!(public_key(&key).unwrap(), generated.public_key);
        key.private_key = Some("garbage".into());
        assert!(public_key(&key).unwrap_err().starts_with("Unable to export the public key"));
    }

    fn export(script: &str) -> Result<(), String> {
        let dir = tempfile::tempdir().unwrap();
        let mut target = SshTarget::new(&format!("127.0.0.1:{}", server::start()), Some("root".into()));
        target.known_hosts = dir.path().join("known_hosts");
        target.identity_files.clear();
        target.password = Some(server::PASSWORD.into());
        let mut export = KeyExport::start(target, script.into());
        let mut events = export.take_events().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let answers = export.answers.clone();
        std::thread::spawn(move || {
            futures::executor::block_on(async {
                while let Some(event) = events.next().await {
                    match event {
                        ExportEvent::Login(LoginEvent::HostKey(_)) => {
                            let _ = answers.unbounded_send(Answer::HostKey(HostKeyDecision::Once));
                        }
                        ExportEvent::Done(result) => {
                            let _ = tx.send(result);
                        }
                        ExportEvent::Login(_) => {}
                    }
                }
            });
        });
        rx.recv_timeout(Duration::from_secs(10)).expect("no answer")
    }

    #[test]
    fn export_succeeds_on_exit_status_zero() {
        assert_eq!(export("echo $SHELL"), Ok(()));
    }

    #[test]
    fn export_reports_the_script_output_on_failure() {
        assert_eq!(export("fail:Permission denied"), Err("Permission denied".into()));
    }
}
