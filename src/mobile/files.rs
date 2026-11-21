use std::path::PathBuf;

use futures::channel::mpsc::UnboundedReceiver;
use serde_json::{json, Value};

use super::text;
use crate::{
    files::{self, Entry},
    remote_fs::{Operation, RemoteEvent, RemoteFs, TransferId},
    session::SshTarget,
    transfer::{Control, Destination, Direction, Item, Job, Planned, TransferEvent},
};

pub(super) struct Browser {
    remote: RemoteFs,
    events: UnboundedReceiver<RemoteEvent>,
    panel: files::Panel,
    ready: bool,
    loading: bool,
    temp: tempfile::TempDir,
    transfer: Option<(TransferId, bool, String, bool)>,
    export: Option<(String, bool)>,
}

impl Browser {
    pub(super) fn new(target: SshTarget, root: PathBuf, show_hidden: bool) -> Result<Self, String> {
        let temp = tempfile::Builder::new()
            .prefix("sftp-")
            .tempdir_in(root)
            .map_err(|e| e.to_string())?;
        let mut remote = RemoteFs::connect(target);
        let events = remote
            .take_events()
            .ok_or("Could not display SFTP events.")?;
        let mut panel = files::Panel::new("/");
        panel.show_hidden = show_hidden;
        Ok(Self {
            remote,
            events,
            panel,
            ready: false,
            loading: true,
            temp,
            transfer: None,
            export: None,
        })
    }

    fn refresh(&mut self) {
        self.loading = true;
        self.remote.list(&self.panel.path);
    }

    fn remove_stage(&self, name: &str, tree: bool) {
        let path = self.temp.path().join(name);
        let _ = if tree {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        };
    }

    fn listing(&self) -> Value {
        json!({"type":"listing", "path":self.panel.path,
            "canBack":self.panel.can_go_back(), "canForward":self.panel.can_go_forward(),
            "entries":self.panel.rows().iter().map(|e| json!({
                "name":e.name, "folder":e.is_dir, "link":e.is_link, "hidden":e.is_hidden(),
                "size":e.size_label(), "modified":e.modified_label(),
                "mode":e.mode.map(|n| format!("{:03o}", n & 0o777))
            })).collect::<Vec<_>>()
        })
    }

    pub(super) fn call(&mut self, op: &str, args: &Value) -> Result<Value, String> {
        if op == "filesHidden" {
            self.panel.show_hidden = args["enabled"] == true;
            return Ok(if self.ready && !self.loading { self.listing() } else { json!({}) });
        }
        if op == "filesAnswer" {
            self.remote.answer(super::login_answer(args));
            return Ok(json!({}));
        }
        if op == "filesControl" {
            if let Some((id, _, _, _)) = self.transfer.as_ref() {
                let control = if args["retry"] == true {
                    Control::Retry
                } else {
                    Control::Cancel
                };
                self.remote.control(*id, control);
            }
            return Ok(json!({}));
        }
        if op == "filesRelease" {
            if let Some((name, tree)) = self.export.take() {
                self.remove_stage(&name, tree);
            }
            return Ok(json!({}));
        }
        if !self.ready {
            return Err("SFTP is not connected.".into());
        }
        if self.transfer.is_some() || self.export.is_some() {
            return Err("Finish the current file transfer first.".into());
        }
        let entry = || {
            self.panel
                .entry(text(args, "name"))
                .ok_or("File not found.")
        };
        let name = |v| {
            if files::valid_entry_name(v) {
                Ok(v)
            } else {
                Err("Invalid file name.")
            }
        };
        let relative = |e: &Entry| files::join(&self.panel.path, &e.name);
        match op {
            "filesUp" => {
                if self.panel.up() {
                    self.refresh();
                }
            }
            "filesBack" => {
                if self.panel.go_back() {
                    self.refresh();
                }
            }
            "filesForward" => {
                if self.panel.go_forward() {
                    self.refresh();
                }
            }
            "filesRefresh" => self.refresh(),
            "filesFolder" => {
                let e = entry()?;
                if !e.is_dir || e.is_link {
                    return Err("This entry is not a regular folder.".into());
                }
                self.panel.navigate(relative(e));
                self.refresh();
            }
            "filesMkdir" => self.remote.operate(vec![Operation::MakeDir(files::join(
                &self.panel.path,
                name(text(args, "newName"))?,
            ))]),
            "filesRename" => self.remote.operate(vec![Operation::Rename {
                from: relative(entry()?),
                to: files::join(&self.panel.path, name(text(args, "newName"))?),
            }]),
            "filesDelete" => {
                let e = entry()?;
                self.remote.operate(vec![Operation::Remove {
                    path: relative(e),
                    is_dir: e.is_dir && !e.is_link,
                }]);
            }
            "filesChmod" => {
                let e = entry()?;
                if e.is_link {
                    return Err("Cannot edit permissions on a symbolic link.".into());
                }
                let mode = u32::from_str_radix(text(args, "mode"), 8)
                    .ok()
                    .filter(|mode| *mode <= 0o777)
                    .ok_or("Enter an octal mode between 000 and 777.")?;
                self.remote.operate(vec![Operation::Chmod {
                    path: relative(e),
                    mode,
                }]);
            }
            "filesTemp" => {
                let tree = args["tree"] == true;
                let path = self
                    .temp
                    .path()
                    .join(if tree { "incoming-tree" } else { "incoming" });
                if tree {
                    self.remove_stage("incoming-tree", true);
                    std::fs::create_dir(&path).map_err(|e| e.to_string())?;
                }
                return Ok(json!({"path":path}));
            }
            "filesDownload" | "filesUpload" => {
                if self.loading {
                    return Err("Wait for the folder listing.".into());
                }
                let download = op == "filesDownload";
                let (src, dst, title, tree) = if download {
                    let e = entry()?;
                    if e.is_link || !files::valid_local_name(&e.name) {
                        return Err("Select a regular file or folder.".into());
                    }
                    (e.name.clone(), "outgoing".into(), e.name.clone(), e.is_dir)
                } else {
                    let target = name(text(args, "newName"))?.to_owned();
                    let tree = args["tree"] == true;
                    let source = if tree { "incoming-tree" } else { "incoming" };
                    std::fs::symlink_metadata(self.temp.path().join(source))
                        .map_err(|e| e.to_string())
                        .and_then(|m| {
                            if (tree && m.is_dir()) || (!tree && m.is_file()) {
                                Ok(())
                            } else {
                                Err("Upload source changed.".into())
                            }
                        })?;
                    (source.into(), target.clone(), target, tree)
                };
                let local = self.temp.path().to_str().ok_or("Invalid path.")?.to_owned();
                let remote = self.panel.path.clone();
                let (direction, source_dir, destination_dir) = if download {
                    (Direction::Download, remote, local)
                } else {
                    (Direction::Upload, local, remote)
                };
                let destination = if !download && args["replace"] == true {
                    Destination::Replace
                } else {
                    Destination::Create
                };
                let id = self.remote.transfer(Job {
                    direction,
                    source_dir,
                    destination_dir,
                    planned: Planned {
                        item: Item {
                            name: src,
                            is_dir: tree,
                        },
                        target: dst,
                        destination,
                    },
                });
                self.transfer = Some((id, download, title, tree));
            }
            _ => return Err("Unknown file operation.".into()),
        }
        Ok(json!({}))
    }

    pub(super) fn poll(&mut self) -> Value {
        let mut events = Vec::new();
        for _ in 0..32 {
            let Ok(event) = self.events.try_recv() else {
                break;
            };
            match event {
                RemoteEvent::Login(event) => {
                    if let Some(value) = super::login_notice(event) {
                        events.push(value);
                    }
                }
                RemoteEvent::Connected { home } => {
                    self.ready = true;
                    let show_hidden = self.panel.show_hidden;
                    self.panel = files::Panel::new(&home);
                    self.panel.show_hidden = show_hidden;
                    self.refresh();
                    events.push(json!({"type":"connected"}));
                }
                RemoteEvent::Listed { path, result } if path == self.panel.path => {
                    self.loading = false;
                    match result {
                        Ok(entries) => {
                            self.panel.set_entries(entries);
                            events.push(self.listing());
                        }
                        Err(error) => events.push(json!({"type":"error","value":error})),
                    }
                }
                RemoteEvent::Changed(result) => match result {
                    Ok(()) => self.refresh(),
                    Err(error) => events.push(json!({"type":"error","value":error})),
                },
                RemoteEvent::Closed(error) => {
                    self.ready = false;
                    events.push(json!({"type":"closed","value":error.unwrap_or_else(|| "Disconnected".into())}));
                }
                RemoteEvent::Transfer { id, event } => match event {
                    TransferEvent::Progress(p) => {
                        events.push(json!({"type":"progress","value":p.summary(false)}))
                    }
                    TransferEvent::Failed { error, .. } => {
                        events.push(json!({"type":"retry","value":error}))
                    }
                    final_event => {
                        self.remote.finished(id);
                        if let Some((_, download, title, tree)) = self.transfer.take() {
                            if download && final_event == TransferEvent::Finished {
                                self.export = Some(("outgoing".into(), tree));
                                events.push(json!({"type":"export","name":title,"path":self.temp.path().join("outgoing"),"tree":tree}));
                            } else {
                                self.remove_stage(
                                    if tree { "incoming-tree" } else { "incoming" },
                                    tree,
                                );
                                let value = match final_event {
                                    TransferEvent::Finished => "Upload complete".into(),
                                    TransferEvent::Cancelled => "Transfer cancelled".into(),
                                    TransferEvent::CommitFailed { error } => error,
                                    _ => "Transfer incomplete".into(),
                                };
                                events.push(json!({"type":"transferDone","value":value}));
                                if !download {
                                    self.refresh();
                                }
                            }
                        }
                    }
                },
                _ => {}
            }
        }
        json!(events)
    }
}
