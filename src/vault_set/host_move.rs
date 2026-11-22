use crate::{
    language::text as tr,
    vault::{Host, Vault},
};

pub(super) fn insert(source: &Vault, target: &mut Vault, mut host: Host) -> Result<(), String> {
    let missing = || tr("A credential or startup command for this host is missing.").to_owned();

    host.key = host
        .key
        .map(|id| copy_key(source, target, id).ok_or_else(missing))
        .transpose()?;
    if let Some(id) = host.identity {
        let mut identity = source.identity(id).cloned().ok_or_else(missing)?;
        identity.key = identity
            .key
            .map(|key| copy_key(source, target, key).ok_or_else(missing))
            .transpose()?;
        host.identity = Some(
            match target.identities.iter().find(|other| {
                identity.id = other.id;
                **other == identity
            }) {
                Some(existing) => existing.id,
                None => {
                    identity.id = target.next_id();
                    let id = identity.id;
                    target.save_identity(identity);
                    id
                }
            },
        );
    }
    if let Some(id) = host.settings.startup_snippet {
        let snippet = source.snippet(id).ok_or_else(missing)?;
        host.settings.startup_snippet = Some(
            match target
                .snippets
                .iter()
                .find(|other| other.label == snippet.label && other.script == snippet.script)
            {
                Some(existing) => existing.id,
                None => {
                    let mut snippet = snippet.clone();
                    snippet.id = target.next_id();
                    snippet.package = None;
                    snippet.targets.clear();
                    let id = snippet.id;
                    target.save_snippet(snippet);
                    id
                }
            },
        );
    }
    host.group = None;
    host.id = target.next_id();
    target.save_host(host);
    Ok(())
}

fn copy_key(source: &Vault, target: &mut Vault, id: u64) -> Option<u64> {
    let mut key = source.key(id)?.clone();
    if let Some(existing) = target.keys.iter().find(|other| {
        key.id = other.id;
        **other == key
    }) {
        return Some(existing.id);
    }
    key.id = target.next_id();
    let id = key.id;
    target.save_key(key);
    Some(id)
}
