// Read-only comparison of portable self-cloud snapshots.
// Persist only SHA-256 identifiers and content digests, never plaintext secrets.
// Retain the existing on-disk schema identifier for backwards compatibility.
export const SNAPSHOT_FORMAT = 'termius-portable-snapshot';
// Known hosts are device-local and never compared (older snapshots may hold them).
export const COLLECTIONS = Object.freeze({
  groups: 'Группа', hosts: 'Хост', identities: 'Идентификатор',
  sshKeys: 'SSH-ключ', snippets: 'Сниппет', snippetPackages: 'Пакет сниппетов',
  portForwardingRules: 'Проброс портов',
  sshConfigs: 'SSH-конфигурация', telnetConfigs: 'Telnet-конфигурация',
  proxies: 'Прокси', workspaceTemplates: 'Шаблон рабочей области',
  settings: 'Настройки',
});
const NON_SEMANTIC = new Set([
  'local_id', 'id', 'global_id', 'updated_at', 'local_updated_at',
  'created_at', 'changedFields', 'status', 'is_shared', 'encrypted_with',
]);
const redactLabel = value => String(value ?? '').replace(/[\r\n\t\x00-\x1f]/g, ' ').trim().slice(0, 100);

function normalize(value) {
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === 'object') {
    const result = {};
    for (const key of Object.keys(value).sort()) {
      if (!NON_SEMANTIC.has(key) && !['__proto__', 'prototype', 'constructor'].includes(key)) {
        result[key] = normalize(value[key]);
      }
    }
    return result;
  }
  return value;
}
async function sha(text) {
  const buffer = new TextEncoder().encode(text);
  const digest = await crypto.subtle.digest('SHA-256', buffer);
  return Array.from(new Uint8Array(digest), byte => byte.toString(16).padStart(2, '0')).join('');
}

function displayName(kind, record, index) {
  if (kind === 'settings') return 'Параметры приложения';
  const value = record?.label || record?.name || record?.hostnames || record?.address ||
    record?.hostname || record?.bind_address || record?.port ||
    record?.username || record?.public_key_comment;
  if (typeof value === 'string' || typeof value === 'number') {
    const label = redactLabel(value);
    if (label) return label;
  }
  return 'запись ' + (index + 1);
}
function safeFields(kind, record) {
  if (!record || typeof record !== 'object') return {};
  const safe = {};
  const add = (key, value) => {
    if (value != null && (typeof value === 'string' || typeof value === 'number' || typeof value === 'boolean')) {
      safe[key] = redactLabel(value);
    }
  };
  if (kind === 'hosts') {
    add('Название', record.label);
    add('Адрес', record.address);
    add('Порт SSH', record.ssh_config?.port);
    add('Пользователь', record.ssh_config?.identity?.username);
  } else if (kind === 'sshConfigs') {
    add('Порт SSH', record.port);
    add('Кодировка', record.charset);
  } else if (kind === 'portForwardingRules') {
    add('Тип', record.kind || record.type);
    add('Локальный порт', record.port);
    add('Удалённый порт', record.host_port);
  } else if (kind !== 'settings') {
    add('Название', record.label || record.name);
  }
  return safe;
}

export async function indexSnapshot(snapshot) {
  if (snapshot != null && (snapshot.format !== SNAPSHOT_FORMAT || snapshot.version !== 1)) {
    throw new Error('Неподдерживаемый формат снимка облака');
  }
  const entries = new Map();
  if (!snapshot) return entries;
  for (const kind of Object.keys(COLLECTIONS)) {
    const records = kind === 'settings'
      ? (Object.keys(snapshot.settings || {}).length ? [snapshot.settings] : [])
      : snapshot[kind];
    if (!Array.isArray(records)) throw new Error('Неполный снимок: ' + kind);
    const occurrences = new Map();
    for (let index = 0; index < records.length; index++) {
      const record = records[index];
      const label = displayName(kind, record, index);
      const name = kind + ':' + label.toLowerCase();
      const count = occurrences.get(name) || 0;
      occurrences.set(name, count + 1);
      // The importer reassigns local_ids. Hash logical human names instead.
      const key = await sha(name + '#' + count);
      const digest = await sha(JSON.stringify(normalize(record)));
      entries.set(key, { digest, kind, label, fields: safeFields(kind, record) });
    }
  }
  return entries;
}
export function baselineOf(index) {
  return Object.fromEntries([...index].map(([key, item]) => [key, item.digest]));
}
export async function serverKey(url) {
  return sha(String(url || '').trim().replace(/\/+$/, '').toLowerCase());
}
const isHashes = value => !!value && typeof value === 'object' && !Array.isArray(value);
// The baseline records what each side looked like when they were last known
// to hold the same data. After a download the local export is the imported
// cloud snapshot, yet not byte-identical to it (importer defaults), so both
// sides are kept: version 1 stored one set of hashes for both.
export function readBaseline(storage, key) {
  try {
    const parsed = JSON.parse(storage.getItem('termian:self-cloud:baseline:v1') || 'null');
    if (parsed?.server !== key) return null;
    if (parsed.version === 1 && isHashes(parsed.hashes)) {
      return { local: parsed.hashes, remote: parsed.hashes };
    }
    if (parsed.version === 2 && isHashes(parsed.local) && isHashes(parsed.remote)) {
      return { local: parsed.local, remote: parsed.remote };
    }
  } catch (_) { /* never trust a corrupt baseline */ }
  return null;
}
export function saveBaseline(storage, server, localEntries, remoteEntries = localEntries) {
  storage.setItem('termian:self-cloud:baseline:v1', JSON.stringify({
    version: 2, server, local: baselineOf(localEntries), remote: baselineOf(remoteEntries),
  }));
}
function baselineSides(baseline) {
  if (baseline === null || baseline === undefined) return null;
  return isHashes(baseline.local) && isHashes(baseline.remote)
    ? baseline : { local: baseline, remote: baseline };
}
const changeKind = (oldDigest, newDigest) =>
  oldDigest === undefined && newDigest !== undefined ? 'добавлен' :
    newDigest === undefined ? 'удалён' : 'изменён';

export function compareSnapshots(local, cloud, baselineInput, cloudExists = true) {
  const baseline = baselineSides(baselineInput);
  const empty = local.size === 0 && cloud.size === 0;
  const allKeys = new Set([...local.keys(), ...cloud.keys(),
    ...Object.keys(baseline?.local || {}), ...Object.keys(baseline?.remote || {})]);
  const changes = [];
  let upload = 0, download = 0, conflicts = 0, unknown = 0;
  for (const key of allKeys) {
    const left = local.get(key), right = cloud.get(key);
    const l = left?.digest, r = right?.digest;
    if (l === r) continue;
    let direction;
    if (baseline === null) {
      direction = l === undefined ? 'download' : r === undefined ? 'upload' : 'uncertain';
      // Without a synced anchor, simultaneous data is ambiguous (could be a deletion).
      if (local.size && cloud.size) direction = 'uncertain';
    } else {
      const localChanged = l !== baseline.local[key], cloudChanged = r !== baseline.remote[key];
      // Both sides unchanged since they were last in sync: same data.
      if (!localChanged && !cloudChanged) continue;
      direction = localChanged && !cloudChanged ? 'upload' :
        cloudChanged && !localChanged ? 'download' : 'conflict';
    }
    if (direction === 'upload') upload++;
    else if (direction === 'download') download++;
    else if (direction === 'conflict') conflicts++;
    else unknown++;
    const item = left || right;
    const action = direction === 'download'
      ? changeKind(baseline?.remote[key] ?? l, r)
      : changeKind(baseline?.local[key] ?? r, l);
    const subject = (COLLECTIONS[item?.kind] || 'Запись') + ' «' + (item?.label || 'ранее сохранённая запись') + '»';
    const prefix = direction === 'upload' ? 'На устройстве' :
      direction === 'download' ? 'В облаке' :
      direction === 'conflict' ? 'Конфликт' : 'Расхождение';
    const details = [];
    if (left && right && l !== r) {
      const before = direction === 'download' ? left.fields : right.fields;
      const after = direction === 'download' ? right.fields : left.fields;
      for (const field of new Set([...Object.keys(before), ...Object.keys(after)])) {
        if (before[field] !== after[field]) {
          details.push(field + ': ' + (before[field] ?? '—') + ' → ' + (after[field] ?? '—'));
        }
      }
    }
    const detail = left && right && l !== r
      ? (details.length ? ' (' + details.join('; ') + ')' : ' (другие параметры изменены; секреты скрыты)') : '';
    const actionId = direction === 'conflict' ? 'conflict' : direction === 'uncertain' ? 'uncertain' :
      action === 'добавлен' ? 'added' : action === 'удалён' ? 'removed' : 'modified';
    // Structured metadata keeps the preview groupable without parsing Russian
    // sentences. Preserve text for existing callers and diagnostic logs.
    changes.push({ direction, kind: item?.kind, action: actionId,
      label: item?.label || 'ранее сохранённая запись', detail: detail.trim(),
      text: prefix + ': ' + action + ' — ' + subject + detail });
  }
  if (!cloudExists && baseline !== null) {
    conflicts = Math.max(1, conflicts);
    changes.unshift({ direction: 'conflict', action: 'notice', kind: null,
      text: 'Снимок в облаке удалён после предыдущей синхронизации' });
  }
  const kind = conflicts ? 'conflict' : unknown ? 'uncertain' :
    upload && download ? 'conflict' : upload ? 'upload' :
    download ? 'download' : empty ? 'empty' : 'synced';
  return {
    kind, changes: changes.slice(0, 100), total: changes.length,
    upload, download, conflicts, unknown, checkedAt: Date.now(),
  };
}
