// Git-like three-way merge of portable self-cloud snapshots.
//
// Importing a snapshot gives every record a new IndexedDB local_id, so records
// cannot be matched across devices by local_id. The cloud snapshot therefore
// uses stable sync IDs as its `local_id` values (strings for new records, the
// old numbers for snapshots uploaded before this format), and each device keeps
// a map local_id -> sync ID per collection.
//
// The base is two sets of record digests (what this device and the cloud held
// after the last sync), like sync-diff's baseline: a record differs from its
// own side's base only when that side really changed it. Changes on one side
// are applied to the other; the same record changed differently on both sides
// is a conflict that is never resolved silently.
export const KINDS = Object.freeze([
  'groups', 'hosts', 'identities', 'sshKeys', 'snippets', 'snippetPackages',
  'portForwardingRules', 'sshConfigs', 'telnetConfigs', 'proxies', 'workspaceTemplates',
]);
// Reference fields and the collection they point to. Embedded copies are
// rebuilt from the collections by the importer (validateSnapshot), so only
// their IDs matter for merging.
const REF_FIELDS = Object.freeze({
  ssh_key: 'sshKeys', identity: 'identities', proxy: 'proxies', startup_snippet: 'snippets',
  parent_group: 'groups', group: 'groups', ssh_config: 'sshConfigs', telnet_config: 'telnetConfigs',
  package: 'snippetPackages', host: 'hosts',
});
const REF_LISTS = Object.freeze({ targetHosts: 'hosts', hosts_chain: 'hosts' });
// Bookkeeping that differs between devices or after a re-import.
const VOLATILE = new Set(['id', 'global_id', 'updated_at', 'local_updated_at', 'created_at', 'changedFields', 'is_shared', 'encrypted_with']);
const LABELS = Object.freeze({
  groups: 'Группа', hosts: 'Хост', identities: 'Идентификатор', sshKeys: 'SSH-ключ',
  snippets: 'Сниппет', snippetPackages: 'Пакет сниппетов', portForwardingRules: 'Проброс портов',
  sshConfigs: 'SSH-конфигурация', telnetConfigs: 'Telnet-конфигурация', proxies: 'Прокси',
  workspaceTemplates: 'Шаблон рабочей области', settings: 'Настройка',
});
const MERGE_STATE = 'termian:self-cloud:merge:v1';

const isObject = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const token = id => typeof id + ':' + String(id);
const refId = value => isObject(value) && value.local_id != null ? value.local_id : null;
const keyOf = (kind, id) => kind + '|' + token(id);

// Walk a record, replacing the local_id of the record itself and of every
// reference (including references inside embedded copies) with `idOf`.
function mapIds(value, kind, idOf) {
  const walk = (item, itemKind) => {
    if (Array.isArray(item)) return item.map(child => walk(child, null));
    if (!isObject(item)) return item;
    const out = {};
    for (const [key, child] of Object.entries(item)) {
      if (key === 'local_id' && itemKind) out[key] = child == null ? child : idOf(itemKind, child);
      else if (REF_FIELDS[key] && isObject(child)) out[key] = walk(child, REF_FIELDS[key]);
      else if (REF_LISTS[key] && Array.isArray(child)) out[key] = child.map(x => isObject(x) ? walk(x, REF_LISTS[key]) : x);
      else out[key] = walk(child, null);
    }
    return out;
  };
  return walk(value, kind);
}

// Stable JSON for digests: references reduced to their IDs, other nested
// objects' IDs and bookkeeping dropped, keys sorted.
function canonical(value, kind) {
  const walk = (item, itemKind, top) => {
    if (Array.isArray(item)) return item.map(child => walk(child, null, false));
    if (!isObject(item)) return item;
    if (itemKind && !top) return {local_id: item.local_id ?? null};
    const out = {};
    for (const key of Object.keys(item).sort()) {
      if (VOLATILE.has(key) || (top && key === 'status')) continue;
      const child = item[key];
      if (key === 'local_id') { if (itemKind) out[key] = child; continue; }
      if (REF_FIELDS[key] && isObject(child)) out[key] = walk(child, REF_FIELDS[key], false);
      else if (REF_LISTS[key] && Array.isArray(child)) out[key] = child.map(x => isObject(x) ? walk(x, REF_LISTS[key], false) : x);
      else out[key] = walk(child, null, false);
    }
    return out;
  };
  return walk(value, kind, true);
}

function referencesOf(value, kind) {
  const refs = [];
  const walk = (item, itemKind, top) => {
    if (Array.isArray(item)) return item.forEach(child => walk(child, null, false));
    if (!isObject(item)) return;
    if (itemKind && !top && item.local_id != null) refs.push(keyOf(itemKind, item.local_id));
    for (const [key, child] of Object.entries(item)) {
      if (REF_FIELDS[key] && isObject(child)) walk(child, REF_FIELDS[key], false);
      else if (REF_LISTS[key] && Array.isArray(child)) child.forEach(x => walk(x, REF_LISTS[key], false));
      else if (key !== 'local_id') walk(child, null, false);
    }
  };
  walk(value, kind, true);
  return refs;
}

const sortedJson = value => JSON.stringify(value, (_key, item) =>
  isObject(item) ? Object.fromEntries(Object.keys(item).sort().map(key => [key, item[key]])) : item);

async function sha(text) {
  const digest = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text));
  return Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, '0')).join('');
}

const labelOf = (kind, value) => {
  const label = kind === 'settings' ? value : value?.label || value?.name || (kind === 'proxies' ? value?.hostname : '');
  return typeof label === 'string' ? label.replace(/[\x00-\x1f]/g, ' ').trim().slice(0, 100) : '';
};

export function newSyncId() {
  return globalThis.crypto.randomUUID();
}

// Collections whose records have a user-visible name to match by.
const LABELED = ['groups', 'hosts', 'identities', 'sshKeys', 'snippets', 'snippetPackages', 'proxies', 'portForwardingRules', 'workspaceTemplates'];
// Unnamed records are matched through a matched record that references them.
const STRUCTURE = [
  ['hosts', 'ssh_config', 'sshConfigs'], ['hosts', 'telnet_config', 'telnetConfigs'],
  ['groups', 'ssh_config', 'sshConfigs'], ['groups', 'telnet_config', 'telnetConfigs'],
  ['groups', 'parent_group', 'groups'], ['hosts', 'group', 'groups'],
  ['sshConfigs', 'identity', 'identities'], ['telnetConfigs', 'identity', 'identities'],
  ['sshConfigs', 'proxy', 'proxies'], ['sshConfigs', 'startup_snippet', 'snippets'],
  ['telnetConfigs', 'startup_snippet', 'snippets'], ['identities', 'ssh_key', 'sshKeys'],
  ['proxies', 'identity', 'identities'], ['snippets', 'package', 'snippetPackages'],
];

// Bind local records without a sync ID to records of the other snapshot that
// no local record claims yet: first by name, then through references of
// already-bound records. Used for the first sync of a device (its data may
// have come from an older full download) and after imports.
export function matchIds(local, remote, ids) {
  if (!remote) return ids;
  const claimed = new Set();
  for (const kind of KINDS) for (const syncId of Object.values(ids[kind] ?? {})) claimed.add(keyOf(kind, syncId));
  const bind = (kind, localId, syncId) => {
    if (localId == null || syncId == null) return false;
    if (ids[kind]?.[token(localId)] !== undefined || claimed.has(keyOf(kind, syncId))) return false;
    (ids[kind] ??= {})[token(localId)] = syncId;
    claimed.add(keyOf(kind, syncId));
    return true;
  };
  const byName = (kind, records) => {
    const names = new Map(), seen = new Map();
    for (const record of records ?? []) {
      const label = labelOf(kind, record).toLowerCase();
      if (!label) continue;
      const count = seen.get(label) ?? 0;
      seen.set(label, count + 1);
      names.set(label + '#' + count, record);
    }
    return names;
  };
  for (const kind of LABELED) {
    const theirs = byName(kind, remote[kind]);
    for (const [name, record] of byName(kind, local[kind])) {
      const other = theirs.get(name);
      if (other) bind(kind, refId(record), refId(other));
    }
  }
  const remoteById = Object.fromEntries(KINDS.map(kind => [kind, new Map((remote[kind] ?? []).map(record => [token(refId(record)), record]))]));
  for (let changed = true; changed;) {
    changed = false;
    for (const [kind, field, target] of STRUCTURE) {
      for (const record of local[kind] ?? []) {
        const syncId = ids[kind]?.[token(refId(record))];
        const other = syncId === undefined ? null : remoteById[kind].get(token(syncId));
        if (other && bind(target, refId(record[field]), refId(other[field]))) changed = true;
      }
    }
  }
  return ids;
}

// The local export in sync form. Records without a sync ID get a new one;
// map entries of records that no longer exist are dropped.
export function toSyncForm(snapshot, ids) {
  const used = Object.fromEntries(KINDS.map(kind => [kind, {}]));
  const idOf = (kind, id) => {
    const t = token(id);
    const syncId = ids[kind]?.[t] ?? newSyncId();
    (ids[kind] ??= {})[t] = syncId;
    used[kind][t] = syncId;
    return syncId;
  };
  const out = {...snapshot, knownHosts: []};
  for (const kind of KINDS) out[kind] = (snapshot[kind] ?? []).map(record => mapIds(record, kind, idOf));
  for (const kind of KINDS) ids[kind] = used[kind];
  return out;
}

// local_id -> sync ID map after importing a sync-form snapshot: the importer
// reports [sync ID, new local_id] pairs per collection.
export function idsFromImport(imported) {
  const ids = {};
  for (const kind of KINDS) {
    ids[kind] = {};
    for (const [syncId, localId] of imported?.[kind] ?? []) {
      if (syncId != null && localId != null) ids[kind][token(localId)] = syncId;
    }
  }
  return ids;
}

async function recordsOf(snapshot) {
  const records = new Map();
  if (!snapshot) return records;
  for (const kind of KINDS) {
    for (const value of snapshot[kind] ?? []) {
      const id = refId(value);
      if (id == null) continue;
      records.set(keyOf(kind, id), {kind, value, label: labelOf(kind, value),
        digest: await sha(JSON.stringify(canonical(value, kind)))});
    }
  }
  for (const [name, value] of Object.entries(snapshot.settings ?? {})) {
    records.set('settings|' + name, {kind: 'settings', name, value, label: name,
      digest: await sha(sortedJson(value ?? null))});
  }
  return records;
}

export async function digestsOf(snapshot) {
  return Object.fromEntries([...await recordsOf(snapshot)].map(([key, record]) => [key, record.digest]));
}

function assemble(template, records) {
  const out = {...template, knownHosts: [], settings: {}};
  for (const kind of KINDS) out[kind] = [];
  for (const record of records.values()) {
    if (record.kind === 'settings') out.settings[record.name] = record.value;
    else out[record.kind].push(record.value);
  }
  return out;
}

// A record kept on one side may reference a record the other side deleted.
// Keep the referenced record rather than produce a broken snapshot.
function restoreReferences(records, sources) {
  const restored = [];
  for (let changed = true; changed;) {
    changed = false;
    for (const record of [...records.values()]) {
      if (record.kind === 'settings') continue;
      for (const ref of referencesOf(record.value, record.kind)) {
        if (records.has(ref)) continue;
        const found = sources.map(source => source.get(ref)).find(Boolean);
        if (!found) continue;
        records.set(ref, found);
        restored.push(found);
        changed = true;
      }
    }
  }
  return restored;
}

// Unnamed SSH/Telnet configurations are shown by the host or group using them.
function ownerLabels(...sources) {
  const owners = new Map();
  for (const records of sources) {
    for (const record of records.values()) {
      if (!['hosts', 'groups'].includes(record.kind) || !record.label) continue;
      for (const [field, kind] of [['ssh_config', 'sshConfigs'], ['telnet_config', 'telnetConfigs']]) {
        const id = refId(record.value[field]);
        if (id != null && !owners.has(keyOf(kind, id))) owners.set(keyOf(kind, id), record.label);
      }
    }
  }
  return owners;
}

const sameDigests = (a, b) => a.size === b.size && [...a].every(([key, record]) => b.get(key)?.digest === record.digest);

// prefer: 'local' | 'remote' resolves conflicts in favour of one side.
export async function mergeSnapshots({ local, remote, base = null, prefer = null }) {
  const L = await recordsOf(local), R = await recordsOf(remote);
  const baseLocal = base?.local ?? {}, baseRemote = base?.remote ?? {};
  const toLocal = new Map(), toRemote = new Map(), conflicts = [];
  let fromLocal = 0, fromRemote = 0;
  const owners = ownerLabels(L, R);
  for (const key of new Set([...L.keys(), ...R.keys()])) {
    const l = L.get(key), r = R.get(key);
    let choice;
    if (l?.digest === r?.digest) choice = 'same';
    else {
      const localChanged = l?.digest !== baseLocal[key];
      const remoteChanged = r?.digest !== baseRemote[key];
      if (localChanged && !remoteChanged) choice = 'local';
      else if (remoteChanged && !localChanged) choice = 'remote';
      // Neither side changed it since the last sync: an importer difference.
      else if (!localChanged && !remoteChanged) choice = 'same';
      else if (prefer) choice = prefer;
      else {
        const item = l ?? r;
        const label = item.label || owners.get(key) || 'запись';
        conflicts.push({direction: 'conflict', action: 'conflict', kind: item.kind,
          label, detail: !l ? 'удалено на устройстве, изменено в облаке' : !r ? 'изменено на устройстве, удалено в облаке' : 'изменено на обеих сторонах',
          text: 'Конфликт: ' + (LABELS[item.kind] || 'Запись') + ' «' + label + '»'});
        continue;
      }
    }
    const forLocal = choice === 'remote' ? r : l;
    const forRemote = choice === 'local' ? l : r;
    if (choice === 'local') fromLocal++;
    if (choice === 'remote') fromRemote++;
    if (forLocal) toLocal.set(key, forLocal);
    if (forRemote) toRemote.set(key, forRemote);
  }
  if (conflicts.length) return {conflicts, fromLocal, fromRemote};
  restoreReferences(toLocal, [L, R]);
  restoreReferences(toRemote, [R, L]);
  const template = {...(remote ?? {}), ...local, createdAt: new Date().toISOString()};
  return {
    conflicts,
    fromLocal, fromRemote,
    local: assemble(template, toLocal),
    remote: assemble(template, toRemote),
    importNeeded: !sameDigests(toLocal, L),
    uploadNeeded: !remote || !sameDigests(toRemote, R),
  };
}

// One sync round. `saved` is the stored state ({ids, local, remote} digests)
// or null; the callbacks reach the vault, the server and the importer.
// Returns {conflicts} or the new state with what happened.
export async function syncOnce({ saved, prefer = null, exportLocal, importLocal, fetchRemote, upload, backup, validate }) {
  const ids = saved?.ids ?? {};
  const fetched = await fetchRemote();
  const remote = fetched?.exists ? fetched.snapshot : null;
  if (remote) validate(remote);
  // A snapshot deleted on the server is re-created from this device instead
  // of being read as "every record was deleted".
  const base = saved && remote ? {local: saved.local, remote: saved.remote} : null;
  const exported = await exportLocal();
  matchIds(exported, remote, ids);
  const local = toSyncForm(exported, ids);
  const merged = await mergeSnapshots({local, remote, base, prefer});
  if (merged.conflicts.length) {
    return {conflicts: merged.conflicts, state: saved ? {...saved, ids} : null};
  }
  let cloud = remote;
  if (merged.uploadNeeded) {
    cloud = validate(merged.remote);
    await upload(cloud, {revision: fetched?.revision ?? null, create: !remote});
  }
  let settled = {ids, exported, local};
  if (merged.importNeeded) {
    await backup(exported);
    const incoming = validate(merged.local);
    settled = await settleImport(await importLocal(incoming), incoming, exportLocal);
  }
  return {
    conflicts: [],
    state: {ids: settled.ids, local: await digestsOf(settled.local), remote: await digestsOf(cloud)},
    exported: settled.exported, remote: cloud,
    uploaded: merged.uploadNeeded, imported: merged.importNeeded,
    fromLocal: merged.fromLocal, fromRemote: merged.fromRemote,
  };
}

// After importing a sync-form snapshot: map the new local IDs to sync IDs.
export async function settleImport(importedIds, snapshot, exportLocal) {
  const ids = idsFromImport(importedIds);
  const exported = await exportLocal();
  matchIds(exported, snapshot, ids);
  return {ids, exported, local: toSyncForm(exported, ids)};
}

export function readMergeState(storage, server) {
  try {
    const state = JSON.parse(storage.getItem(MERGE_STATE) || 'null');
    if (state?.version === 1 && state.server === server && isObject(state.ids) && isObject(state.local) && isObject(state.remote)) return state;
  } catch (_) { /* a corrupt state only costs a first-sync match */ }
  return null;
}

export function saveMergeState(storage, server, { ids, local, remote }) {
  storage.setItem(MERGE_STATE, JSON.stringify({version: 1, server, ids, local, remote}));
}
