'use strict';

/*
 * Renderer/background-process adapter for portable, decrypted Termius snapshots.
 * It deliberately uses only the public Store methods found in the 9.35.4 bundle.
 * Histories, session logs, known hosts, account/auth state, team metadata,
 * hardware keys and device-bound SSHid/passkeys are never exported.
 * Known hosts are device-local trust decisions: snapshots always carry an empty
 * `knownHosts` array (kept for format compatibility), imports ignore any that an
 * older snapshot contains and never delete the local ones.
 */

const FORMAT = 'termius-portable-snapshot';
const VERSION = 1;
const PORTABLE_SETTING_KEYS = [
  'settings', 'settings_font_size', 'settings_font_family', 'dark_mode',
  'keep_alive_interval',
];
// The background Settings model is the synced user-settings record, not the API
// client. These are bookkeeping fields on that model and must never travel.
const SETTINGS_METADATA_KEYS = new Set(['local_id','id','global_id','created_at','updated_at','local_updated_at','status']);
// user_settings is a separate record from account/auth state. Its schema varies
// across existing profiles: RO consumes only some fields, not the full schema.
// Preserve JSON preferences instead of maintaining an incomplete field whitelist.
const UNSAFE_SETTING_KEYS = new Set(['__proto__', 'prototype', 'constructor']);
const AUTH_SETTING_KEYS = new Set(['auth', 'authtoken', 'accesstoken', 'refreshtoken', 'sessiontoken', 'apitoken', 'token', 'password', 'secret', 'securedata', 'credentials', 'authorization', 'apikey', 'privatekey', 'username', 'usernamestr']);
// Log and connection-history preferences stay on each device.
const LOCAL_ONLY_SETTING = /(^|_)(session_)?logs?($|_)|logging|histor/i;
const ENTITY_METADATA_KEYS = new Set(['id','global_id','updated_at','local_updated_at']);
const installedApis = new WeakSet();

function fail(message) { throw new Error(`Self-cloud snapshot: ${message}`); }
function isObject(value) { return value !== null && typeof value === 'object' && !Array.isArray(value); }
function refId(value) {
  if (!isObject(value)) return value == null ? null : value;
  return Object.prototype.hasOwnProperty.call(value, 'local_id') && value.local_id != null ? value.local_id : null;
}
function plain(value) {
  if (value == null) return value;
  try { return JSON.parse(JSON.stringify(value)); }
  catch (error) { fail(`cannot serialize an entity (${error.message})`); }
}
function portablePlain(value) {
  const raw = plain(value);
  const walk = item => {
    if (Array.isArray(item)) return item.map(walk);
    if (!isObject(item)) return item;
    const out = {};
    const entityLike = Object.prototype.hasOwnProperty.call(item, 'local_id') || Object.prototype.hasOwnProperty.call(item, 'entityName');
    for (const [key, child] of Object.entries(item)) {
      if (key === '__proto__' || key === 'prototype' || key === 'constructor') fail(`unsafe object key ${JSON.stringify(key)}`);
      if (entityLike && ENTITY_METADATA_KEYS.has(key)) continue;
      if (entityLike && key === 'changedFields') { out[key] = []; continue; }
      // Store writes use these flags to choose a vault cryptor. Portable data is
      // always re-created as personal/local data under the destination key.
      if (key === 'is_shared') { out[key] = false; continue; }
      if (key === 'encrypted_with') { out[key] = null; continue; }
      out[key] = walk(child);
    }
    return out;
  };
  return walk(raw);
}
function portableEntity(value) {
  const out = portablePlain(value);
  if (isObject(out)) delete out.status; // entity sync status; nested protocol statuses remain semantic
  return out;
}
function validateSettings(settings) {
  if (!isObject(settings)) fail('snapshot.settings must be an object');
  const copy = (value, depth) => {
    if (depth > 64) fail('user settings exceed maximum nesting depth');
    if (value === null || typeof value === 'string' || typeof value === 'boolean') return value;
    if (typeof value === 'number' && Number.isFinite(value)) return value;
    if (Array.isArray(value)) return value.map(item => copy(item, depth + 1));
    if (!isObject(value) || ![Object.prototype, null].includes(Object.getPrototypeOf(value))) fail('user settings must contain only JSON data');
    const out = {};
    for (const [key, child] of Object.entries(value)) {
      if (UNSAFE_SETTING_KEYS.has(key) || AUTH_SETTING_KEYS.has(key.toLowerCase().replace(/[\s_-]/g, ''))) fail(`unsupported user setting ${JSON.stringify(key)}`);
      if (depth === 0 && (SETTINGS_METADATA_KEYS.has(key) || LOCAL_ONLY_SETTING.test(key))) continue;
      out[key] = copy(child, depth + 1);
    }
    return out;
  };
  return copy(settings, 0);
}
function list(value, name) {
  if (!Array.isArray(value)) fail(`snapshot.${name} must be an array`);
  return value;
}
function errorText(error) { return error instanceof Error ? error.message : String(error); }

function detectUnsupported(store, data) {
  if (typeof store.getAllSepKeys !== 'function') fail('this build cannot report hardware-backed keys safely');
  if (typeof store.getAllTeamMultiKey !== 'function') fail('this build cannot report team multikey records safely');
  const sepKeys = store.getAllSepKeys();
  if (sepKeys && sepKeys.length) fail('hardware-backed keys are present; they cannot be made portable');
  const multiKeys = store.getAllTeamMultiKey();
  if (multiKeys && multiKeys.length) fail('team multikey records are present; this snapshot format does not support them');
  const collections = store.collections || {};
  if (collections.port_knockings && collections.port_knockings.length) fail('port-knocking records are present; this snapshot format does not support them');
  const cloudCollectionCount = ['aws_configs','digital_ocean_configs','azure_configs'].reduce((total, name) => total + ((collections[name] && collections[name].length) || 0), 0);
  const attachedCloudConfigs = data.groups.filter(group => group.cloud_config != null).length;
  if (cloudCollectionCount > attachedCloudConfigs) fail('an unattached cloud-provider configuration is present and cannot be represented portably');
  // Software SSH certificates are portable when attached to an exported SSH key.
  if (typeof store.getSshCertificates === 'function') {
    const attached = new Set(data.sshKeys.flatMap(key => (key.certificates || []).map(refId).filter(id => id != null).map(id => `${typeof id}:${String(id)}`)));
    const orphan = store.getSshCertificates().find(cert => !attached.has(`${typeof refId(cert)}:${String(refId(cert))}`));
    if (orphan) fail(`SSH certificate ${String(refId(orphan))} is not attached to an exported software key`);
  }

  const visit = (value, path, seen = new Set()) => {
    if (!value || typeof value !== 'object' || seen.has(value)) return;
    seen.add(value);
    if (value.hardware_key != null) fail(`${path} uses a hardware-backed key`);
    if (value.sshid_mode || value.sshid_username || value.sshid_all_devices_biometric ||
        (Array.isArray(value.sshid_passkeys_with_meta) && value.sshid_passkeys_with_meta.length)) {
      fail(`${path} uses SSHid/device-bound passkeys`);
    }
    if (value.keyVariant === 'fido') fail(`${path} is a FIDO/security-key private key`);
    for (const [key, child] of Object.entries(value)) {
      if (key === 'encrypted_with') continue; // vault metadata; fields returned by Store are decrypted
      if (isObject(child) && (child.decryptionAlgorithm || child.ciphertext || child.encryptedData)) {
        fail(`${path}.${key} is still encrypted or could not be decrypted`);
      }
      visit(child, `${path}.${key}`, seen);
    }
  };
  for (const [name, values] of Object.entries(data)) for (const value of values) visit(value, name);
}

function assertCollectionCompleteness(store, data) {
  const mappings = {
    groups: 'groups', hosts: 'hosts', identities: 'identities', sshKeys: 'ssh_keys',
    snippets: 'snippets', snippetPackages: 'snippets_packages',
    portForwardingRules: 'pf_rules',
    sshConfigs: 'ssh_configs', telnetConfigs: 'telnet_configs', proxies: 'proxies',
    workspaceTemplates: 'workspace_templates',
  };
  for (const [name, collectionName] of Object.entries(mappings)) {
    const collection = store.collections && store.collections[collectionName];
    if (collection && typeof collection.length === 'number' && collection.length !== data[name].length) {
      fail(`${collectionName} contains ${collection.length} records but only ${data[name].length} could be read; decryption or mapping failed`);
    }
  }
  const hostChains = store.collections && store.collections.host_chains;
  if (hostChains && typeof hostChains.length === 'number') {
    const represented = new Set(data.sshConfigs.map(config => config.host_chain).filter(Boolean).map(chain => `${typeof refId(chain)}:${String(refId(chain))}`));
    if (hostChains.length !== represented.size) fail(`host_chains contains ${hostChains.length} records but only ${represented.size} could be read through SSH configs`);
  }
  const certificates = store.collections && store.collections.ssh_certificates;
  if (certificates && typeof certificates.length === 'number') {
    const readable = new Set(data.sshKeys.flatMap(key => key.certificates || []).map(refId).filter(id => id != null).map(id => `${typeof id}:${String(id)}`));
    if (certificates.length !== readable.size) fail(`ssh_certificates contains ${certificates.length} records but only ${readable.size} could be read through SSH keys`);
  }
}

function exportSettings(api) {
  if (api.settings && typeof api.settings.getSettings === 'function') {
    const model = api.settings.getSettings();
    if (!model || typeof model.toJSON !== 'function') fail('user settings model is unavailable');
    return validateSettings(model.toJSON());
  }
  if (api.settings && typeof api.settings.exportPortableSettings === 'function') {
    return validateSettings(api.settings.exportPortableSettings());
  }
  const storage = globalThis.localStorage;
  if (!storage || typeof storage.getItem !== 'function') return {};
  const result = {};
  for (const key of PORTABLE_SETTING_KEYS) {
    const value = storage.getItem(key);
    if (value != null) result[key] = value;
  }
  return result;
}

async function importSettings(api, settings) {
  settings = validateSettings(settings);
  if (api.settings && typeof api.settings.setSettings === 'function') {
    const model = typeof api.settings.getSettings === 'function' ? api.settings.getSettings() : null;
    if (model && typeof model.unset === 'function') {
      const previous = typeof model.toJSON === 'function' ? validateSettings(model.toJSON()) : {};
      const local = typeof model.toJSON === 'function' ? model.toJSON() : {};
      for (const key of Object.keys(previous)) if (!Object.prototype.hasOwnProperty.call(settings, key)) model.unset(key);
      // validateSettings dropped the device-local keys; keep their local values.
      for (const key of Object.keys(isObject(local) ? local : {})) if (LOCAL_ONLY_SETTING.test(key) && !SETTINGS_METADATA_KEYS.has(key)) settings[key] = local[key];
    }
    await api.settings.setSettings(plain(settings));
    return;
  }
  if (api.settings && typeof api.settings.importPortableSettings === 'function') {
    await api.settings.importPortableSettings(plain(settings));
    return;
  }
  const storage = globalThis.localStorage;
  if (!storage || typeof storage.setItem !== 'function') {
    if (Object.keys(settings).length) fail('this build cannot restore portable user settings');
    return;
  }
  for (const key of Object.keys(settings)) {
    if (!PORTABLE_SETTING_KEYS.includes(key)) fail(`unsupported user setting ${JSON.stringify(key)}`);
  }
  for (const key of PORTABLE_SETTING_KEYS) {
    if (Object.prototype.hasOwnProperty.call(settings, key)) storage.setItem(key, String(settings[key]));
    else if (typeof storage.removeItem === 'function') storage.removeItem(key);
  }
}

function collect(api) {
  const s = api.store;
  if (!s) fail('Store is unavailable');
  if (typeof s.getProxies !== 'function') fail('this build cannot report proxies safely');
  if (!s.workspaceTemplatesRepository || typeof s.workspaceTemplatesRepository.getMappedWorkspaceTemplates !== 'function') fail('this build cannot report workspace templates safely');
  const data = {
    groups: s.getGroups(),
    hosts: s.getHosts(),
    identities: s.getAllIdentities(),
    sshKeys: s.getSshKeys(),
    snippets: s.getSnippets(),
    snippetPackages: s.getSnippetsPackages(),
    portForwardingRules: s.getPortForwardingRules(),
    knownHosts: [],
    sshConfigs: s.getSshConfigs(),
    telnetConfigs: s.getTelnetConfigs(),
    proxies: s.getProxies(),
    workspaceTemplates: s.workspaceTemplatesRepository.getMappedWorkspaceTemplates(),
  };
  assertCollectionCompleteness(s, data);
  detectUnsupported(s, data);
  return Object.fromEntries(Object.entries(data).map(([key, value]) => [key, value.map(portableEntity)]));
}

function validateSnapshot(snapshot) {
  if (!isObject(snapshot)) fail('snapshot must be an object');
  if (snapshot.format !== FORMAT || snapshot.version !== VERSION) fail(`unsupported format/version (expected ${FORMAT} v${VERSION})`);
  const names = ['groups','hosts','identities','sshKeys','snippets','snippetPackages','portForwardingRules','knownHosts','sshConfigs','telnetConfigs','proxies','workspaceTemplates'];
  const arrays = Object.fromEntries(names.map(name => [name, list(snapshot[name], name)]));
  arrays.knownHosts = []; // device-local; ignored even when an older snapshot has them
  if (!isObject(snapshot.settings)) fail('snapshot.settings must be an object');

  const indexes = {};
  for (const [name, values] of Object.entries(arrays)) {
    const ids = new Map();
    values.forEach((value, index) => {
      if (!isObject(value)) fail(`${name}[${index}] must be an object`);
      const id = refId(value);
      if (!((typeof id === 'number' && Number.isSafeInteger(id)) || (typeof id === 'string' && id.length > 0))) fail(`${name}[${index}].local_id must be a safe integer or non-empty string`);
      const forbidden = ['id','global_id','status'].find(key => Object.prototype.hasOwnProperty.call(value, key));
      if (forbidden) fail(`${name}[${index}] contains official-sync field ${forbidden}`);
      const token = `${typeof id}:${String(id)}`;
      if (ids.has(token)) fail(`${name} contains duplicate id ${String(id)}`);
      ids.set(token, value);
    });
    indexes[name] = ids;
  }
  const has = (kind, value, path, optional = true) => {
    const id = refId(value);
    if (id == null) {
      if (value != null || !optional) fail(`${path} is missing a valid local_id reference`);
      return;
    }
    if (!indexes[kind].has(`${typeof id}:${String(id)}`)) fail(`${path} references missing ${kind} id ${String(id)}`);
  };
  const configRefs = (config, path) => {
    if (!config) return;
    if (!isObject(config)) fail(`${path} must be an object`);
    has('identities', config.identity, `${path}.identity`);
    has('proxies', config.proxy, `${path}.proxy`);
    has('snippets', config.startup_snippet, `${path}.startup_snippet`);
    if (config.host_chain != null) {
      const chain = config.host_chain.hosts_chain;
      if (!Array.isArray(chain)) fail(`${path}.host_chain.hosts_chain must be an array`);
      chain.forEach((host, i) => has('hosts', host, `${path}.host_chain.hosts_chain[${i}]`, false));
    }
  };
  arrays.identities.forEach((x,i) => has('sshKeys', x.ssh_key, `identities[${i}].ssh_key`));
  arrays.proxies.forEach((x,i) => has('identities', x.identity, `proxies[${i}].identity`));
  arrays.sshConfigs.forEach((x,i) => configRefs(x, `sshConfigs[${i}]`));
  arrays.telnetConfigs.forEach((x,i) => configRefs(x, `telnetConfigs[${i}]`));
  arrays.groups.forEach((x,i) => { has('groups', x.parent_group, `groups[${i}].parent_group`); has('sshConfigs', x.ssh_config, `groups[${i}].ssh_config`); has('telnetConfigs', x.telnet_config, `groups[${i}].telnet_config`); configRefs(x.ssh_config, `groups[${i}].ssh_config`); configRefs(x.telnet_config, `groups[${i}].telnet_config`); });
  arrays.hosts.forEach((x,i) => { has('groups', x.group, `hosts[${i}].group`); has('sshConfigs', x.ssh_config, `hosts[${i}].ssh_config`); has('telnetConfigs', x.telnet_config, `hosts[${i}].telnet_config`); configRefs(x.ssh_config, `hosts[${i}].ssh_config`); configRefs(x.telnet_config, `hosts[${i}].telnet_config`); });
  arrays.snippets.forEach((x,i) => {
    has('snippetPackages', x.package, `snippets[${i}].package`);
    if (x.targetHosts != null) {
      if (!Array.isArray(x.targetHosts)) fail(`snippets[${i}].targetHosts must be an array`);
      x.targetHosts.forEach((host,j) => has('hosts', host, `snippets[${i}].targetHosts[${j}]`, false));
    }
  });
  arrays.portForwardingRules.forEach((x,i) => has('hosts', x.host, `portForwardingRules[${i}].host`, false));
  arrays.workspaceTemplates.forEach((x,i) => {
    if (!Array.isArray(x.workspaceTerminals)) fail(`workspaceTemplates[${i}].workspaceTerminals must be an array`);
    x.workspaceTerminals.forEach((terminal,j) => has('hosts', terminal.host, `workspaceTemplates[${i}].workspaceTerminals[${j}].host`, false));
  });

  // Parent cycles would make delete/import ordering unsafe.
  const visiting = new Set(), done = new Set();
  const groupById = id => indexes.groups.get(`${typeof id}:${String(id)}`);
  const walk = group => {
    const id = refId(group), token = `${typeof id}:${String(id)}`;
    if (visiting.has(token)) fail(`group parent cycle at id ${String(id)}`);
    if (done.has(token)) return;
    visiting.add(token);
    const parent = refId(group.parent_group); if (parent != null) walk(groupById(parent));
    visiting.delete(token); done.add(token);
  };
  arrays.groups.forEach(walk);
  detectUnsupported({ getAllSepKeys: () => [], getAllTeamMultiKey: () => [] }, arrays);

  // Replace every relationship payload with one canonical representation. This
  // prevents a snapshot from smuggling two different objects under one local_id.
  const normalized = Object.fromEntries(Object.entries(arrays).map(([name, values]) => [name, portablePlain(values)]));
  const normalizedIndexes = Object.fromEntries(Object.entries(normalized).map(([name, values]) => [name, new Map(values.map(value => [`${typeof refId(value)}:${String(refId(value))}`, value]))]));
  const canonical = (kind, value, minimal = false) => {
    const id = refId(value); if (id == null) return null;
    const found = normalizedIndexes[kind].get(`${typeof id}:${String(id)}`);
    return minimal ? { local_id: found.local_id } : plain(found);
  };
  const normalizeConfig = config => {
    if (!config) return;
    if (config.identity) config.identity = canonical('identities', config.identity);
    if (config.proxy) config.proxy = canonical('proxies', config.proxy);
    if (config.startup_snippet) config.startup_snippet = canonical('snippets', config.startup_snippet, true);
    if (config.host_chain) config.host_chain.hosts_chain = config.host_chain.hosts_chain.map(host => canonical('hosts', host, true));
  };
  normalized.identities.forEach(x => { if (x.ssh_key) x.ssh_key = canonical('sshKeys', x.ssh_key, true); });
  normalized.proxies.forEach(x => { if (x.identity) x.identity = canonical('identities', x.identity); });
  normalized.sshConfigs.forEach(normalizeConfig); normalized.telnetConfigs.forEach(normalizeConfig);
  const canonicalLinkedConfig = (kind, config) => {
    if (!config) return null;
    const status = Object.prototype.hasOwnProperty.call(config, 'status') ? config.status : undefined;
    const result = canonical(kind, config);
    // A config's nested protocol status is semantic, while the same key on the
    // top-level synchronized entity is bookkeeping removed by portableEntity.
    if (status !== undefined) result.status = status;
    return result;
  };
  normalized.groups.forEach(x => { if (x.parent_group) x.parent_group = canonical('groups', x.parent_group, true); if (x.ssh_config) x.ssh_config = canonicalLinkedConfig('sshConfigs', x.ssh_config); if (x.telnet_config) x.telnet_config = canonicalLinkedConfig('telnetConfigs', x.telnet_config); });
  normalized.hosts.forEach(x => { if (x.group) x.group = canonical('groups', x.group, true); if (x.ssh_config) x.ssh_config = canonicalLinkedConfig('sshConfigs', x.ssh_config); if (x.telnet_config) x.telnet_config = canonicalLinkedConfig('telnetConfigs', x.telnet_config); });
  normalized.snippets.forEach(x => { if (x.package) x.package = canonical('snippetPackages', x.package, true); if (x.targetHosts) x.targetHosts = x.targetHosts.map(host => canonical('hosts', host, true)); });
  normalized.portForwardingRules.forEach(x => { x.host = canonical('hosts', x.host, true); });
  normalized.workspaceTemplates.forEach(x => { x.workspaceTerminals.forEach(terminal => { terminal.host = canonical('hosts', terminal.host, true); }); });
  return { ...normalized, settings: validateSettings(snapshot.settings) };
}

async function exportSnapshot(api) {
  const snapshot = {
    format: FORMAT,
    version: VERSION,
    createdAt: new Date().toISOString(),
    scope: {
      included: ['groups','hosts','identities','software SSH keys and certificates','SSH/Telnet configs','proxies','snippets and packages','snippet target hosts','port forwarding rules','tags','environment variables','host chains','workspace templates','portable local user settings'],
      excluded: ['history','session logs','known hosts','account/auth state'],
      rejectedWhenPresent: ['hardware/FIDO keys','SSHyd device-bound passkeys','team multikeys','undecryptable fields','orphan SSH certificates','unattached cloud configurations'],
    },
    ...collect(api),
    settings: exportSettings(api),
  };
  const validated = validateSnapshot(snapshot); // never upload an internally broken graph
  return { ...snapshot, ...validated };
}

async function clearPortableData(store) {
  const templateRepository = store.workspaceTemplatesRepository;
  if (templateRepository && typeof templateRepository.getMappedWorkspaceTemplates === 'function' && typeof templateRepository.delete === 'function') {
    for (const value of templateRepository.getMappedWorkspaceTemplates()) await templateRepository.delete({ local_id: value.local_id });
  }
  const pfs = store.getPortForwardingRules();
  for (const value of pfs) await store.deletePortForwardingRule(value);
  const hosts = store.getHosts();
  if (hosts.length) await store.deleteHosts(hosts);
  const groups = store.getGroups();
  const roots = groups.filter(group => refId(group.parent_group) == null);
  for (const group of roots) await store.deleteGroup(group);
  for (const value of store.getSnippets()) await store.deleteSnippet(value);
  for (const value of store.getSnippetsPackages()) await store.deleteSnippetsPackage(value);
  const sshConfigs = store.getSshConfigs(); if (sshConfigs.length) await store.deleteSshConfig(...sshConfigs);
  const telnetConfigs = store.getTelnetConfigs(); if (telnetConfigs.length) await store.deleteTelnetConfig(...telnetConfigs);
  const proxies = typeof store.getProxies === 'function' ? store.getProxies() : [];
  if (proxies.length) await store.deleteProxy(...proxies);
  const identities = store.getAllIdentities(); if (identities.length) await store.deleteIdentity(...identities);
  for (const value of store.getSshKeys()) await store.deleteSshKey(value);
}

function groupDepth(group, byId) {
  let depth = 0, current = group;
  while (refId(current.parent_group) != null) { current = byId.get(`${typeof refId(current.parent_group)}:${String(refId(current.parent_group))}`); depth++; }
  return depth;
}

async function writePortableData(api, data, models) {
  const s = api.store;
  const token = value => `${typeof refId(value)}:${String(refId(value))}`;
  const maps = Object.fromEntries(['groups','hosts','identities','sshKeys','snippets','snippetPackages','sshConfigs','telnetConfigs','proxies'].map(name => [name, new Map()]));
  // [snapshot local_id, new local_id] per collection: self-cloud merging keeps
  // stable sync IDs in the snapshot and maps them to this device's IDs.
  const created = { portForwardingRules: [], workspaceTemplates: [] };
  for (const name of Object.keys(maps)) created[name] = [];
  const lookup = (kind, value) => {
    if (value == null) return null;
    const found = maps[kind].get(token(value));
    if (!found) fail(`internal import ordering error: unresolved ${kind} id ${String(refId(value))}`);
    return found;
  };
  const clearID = value => {
    if (!value) return value;
    value.local_id = undefined;
    value.id = undefined;
    return value;
  };
  const make = (name, raw) => clearID(new models[name](plain(raw)));
  const saveNew = async (kind, name, raw, save, prepare) => {
    const oldToken = token(raw);
    const value = make(name, prepare ? prepare(raw) : raw);
    await save(value);
    if (refId(value) == null) fail(`${name} save did not assign a local_id`);
    maps[kind].set(oldToken, value);
    created[kind].push([refId(raw), refId(value)]);
    return value;
  };
  const remapConfig = (raw, includeHostChain = false) => {
    if (!raw) return null;
    const value = plain(raw);
    value.identity = raw.identity ? lookup('identities', raw.identity) : null;
    value.proxy = raw.proxy ? lookup('proxies', raw.proxy) : null;
    value.startup_snippet = raw.startup_snippet ? lookup('snippets', raw.startup_snippet) : null;
    if (includeHostChain && raw.host_chain) {
      value.host_chain = new models.HostChain({
        ...plain(raw.host_chain),
        local_id: undefined,
        id: undefined,
        hosts_chain: raw.host_chain.hosts_chain.map(host => lookup('hosts', host)),
      });
    } else value.host_chain = null;
    return value;
  };
  const attachHostChain = (config, raw) => {
    if (!config || !raw || !raw.host_chain) return false;
    config.host_chain = new models.HostChain({
      ...plain(raw.host_chain),
      local_id: undefined,
      id: undefined,
      hosts_chain: raw.host_chain.hosts_chain.map(host => lookup('hosts', host)),
    });
    return true;
  };

  for (const raw of data.sshKeys) {
    const value = make('SshKey', raw);
    for (const certificate of value.certificates || []) clearID(certificate);
    await s.setSshKey(value);
    if (refId(value) == null) fail('SshKey save did not assign a local_id');
    maps.sshKeys.set(token(raw), value);
    created.sshKeys.push([refId(raw), refId(value)]);
  }
  for (const raw of data.identities) await saveNew('identities', 'Identity', raw, value => s.setIdentity(value), value => ({ ...plain(value), ssh_key: value.ssh_key ? lookup('sshKeys', value.ssh_key) : null }));
  for (const raw of data.snippetPackages) await saveNew('snippetPackages', 'SnippetPackage', raw, value => s.setSnippetsPackage(value));
  // Target-host link rows cannot be created until hosts have their destination IDs.
  for (const raw of data.snippets) await saveNew('snippets', 'Snippet', raw, value => s.setSnippet(value), value => ({ ...plain(value), package: value.package ? lookup('snippetPackages', value.package) : null, targetHosts: [] }));
  for (const raw of data.proxies) await saveNew('proxies', 'Proxy', raw, value => s.setProxy(value), value => ({ ...plain(value), identity: value.identity ? lookup('identities', value.identity) : null }));
  for (const raw of data.sshConfigs) await saveNew('sshConfigs', 'SshConfig', raw, value => s.setSshConfig(value, null), value => remapConfig(value));
  for (const raw of data.telnetConfigs) await saveNew('telnetConfigs', 'TelnetConfig', raw, value => s.setTelnetConfig(value, null), value => remapConfig(value));

  const byGroup = new Map(data.groups.map(x => [token(x), x]));
  const groups = [...data.groups].sort((a,b) => groupDepth(a, byGroup) - groupDepth(b, byGroup));
  for (const raw of groups) {
    await saveNew('groups', 'Group', raw, value => s.setGroup(value), value => ({
      ...plain(value),
      parent_group: value.parent_group ? lookup('groups', value.parent_group) : null,
      ssh_config: value.ssh_config ? lookup('sshConfigs', value.ssh_config) : null,
      telnet_config: value.telnet_config ? lookup('telnetConfigs', value.telnet_config) : null,
      cloud_config: value.cloud_config ? { ...plain(value.cloud_config), local_id: undefined, id: undefined } : null,
    }));
  }
  for (const raw of data.hosts) {
    await saveNew('hosts', 'Host', raw, value => s.setHost(value), value => ({
      ...plain(value),
      group: value.group ? lookup('groups', value.group) : null,
      ssh_config: value.ssh_config ? lookup('sshConfigs', value.ssh_config) : null,
      telnet_config: value.telnet_config ? lookup('telnetConfigs', value.telnet_config) : null,
    }));
  }

  // Attach cyclic/forward host relationships using the already-created model
  // instances. Keeping their newly assigned IDs avoids duplicate configs/entities.
  for (const raw of data.snippets) {
    const value = lookup('snippets', raw);
    value.targetHosts = (raw.targetHosts || []).map(host => lookup('hosts', host));
    await s.setSnippet(value);
  }
  for (const raw of data.sshConfigs) {
    const value = lookup('sshConfigs', raw);
    if (attachHostChain(value, raw)) await s.setSshConfig(value, null);
  }
  for (const raw of data.portForwardingRules) {
    const value = make('PortForwardingRule', { ...plain(raw), host: lookup('hosts', raw.host) });
    await s.setPortForwardingRule(value);
    if (refId(value) != null) created.portForwardingRules.push([refId(raw), refId(value)]);
  }
  if (data.workspaceTemplates.length) {
    const repository = s.workspaceTemplatesRepository;
    if (!repository || typeof repository.save !== 'function') fail('this build cannot restore workspace templates');
    for (const raw of data.workspaceTemplates) {
      const value = clearID({
        ...plain(raw),
        workspaceTerminals: raw.workspaceTerminals.map(terminal => ({ ...plain(terminal), host: lookup('hosts', terminal.host) })),
      });
      await repository.save(value);
      if (refId(value) != null) created.workspaceTemplates.push([refId(raw), refId(value)]);
    }
  }
  await importSettings(api, data.settings);
  return created;
}

function preflightImport(api, data) {
  if (data.workspaceTemplates.length) {
    const repository = api.store && api.store.workspaceTemplatesRepository;
    if (!repository || typeof repository.save !== 'function' || typeof repository.delete !== 'function') fail('this build cannot restore workspace templates');
  }
  const settings = api.settings;
  const storage = globalThis.localStorage;
  if (Object.keys(data.settings).length && !(settings && (typeof settings.setSettings === 'function' || typeof settings.importPortableSettings === 'function')) && !(storage && typeof storage.setItem === 'function')) {
    fail('this build cannot restore portable user settings');
  }
}

async function verifyImportedData(api, expected) {
  const actual = await exportSnapshot(api);
  const names = ['groups','hosts','identities','sshKeys','snippets','snippetPackages','portForwardingRules','knownHosts','sshConfigs','telnetConfigs','proxies','workspaceTemplates'];
  for (const name of names) if (actual[name].length !== expected[name].length) {
    fail(`import verification failed for ${name}: wrote ${expected[name].length} but read back ${actual[name].length}`);
  }
}

async function replaceSnapshot(api, snapshot, models, options = {}) {
  const validated = validateSnapshot(snapshot); // absolutely no mutation before this completes
  preflightImport(api, validated);
  const backup = options.rollback === false ? null : await exportSnapshot(api);
  try {
    await clearPortableData(api.store);
    const ids = await writePortableData(api, validated, models);
    await verifyImportedData(api, validated);
    if (typeof options.refresh === 'function') await options.refresh();
    return { ids };
  } catch (error) {
    if (!backup) throw error;
    try {
      const rollbackData = validateSnapshot(backup);
      await clearPortableData(api.store);
      await writePortableData(api, rollbackData, models);
      if (typeof options.refresh === 'function') await options.refresh();
    } catch (rollbackError) {
      throw new Error(`Import failed: ${errorText(error)}; rollback also failed: ${errorText(rollbackError)}`);
    }
    throw new Error(`Import failed and previous local snapshot was restored: ${errorText(error)}`);
  }
}

// Settings → «Логи сессий»: delete connection-history entries (and with them
// their log files) that started at or after `since` (0 = all). Returns the log
// names of the deleted and the remaining entries for the native cleanup.
async function clearSessionLogs(api, since) {
  if (!Number.isFinite(since) || since < 0) fail('invalid log cleanup period');
  const store = api.store;
  if (!store || typeof store.getHistoryEntities !== 'function' || typeof store.deleteHistoryEntity !== 'function') fail('this build cannot delete connection history');
  const logName = history => (history.session_log_data || (history.command && history.command.session_log_data) || {}).name;
  const histories = await store.getHistoryEntities();
  const removed = [], kept = [];
  let entries = 0;
  for (const history of histories) {
    const started = new Date(history.connected_at ?? 0).getTime() || 0;
    if (since === 0 || started >= since) {
      await store.deleteHistoryEntity(history);
      entries++;
      if (logName(history)) removed.push(logName(history));
    } else if (logName(history)) kept.push(logName(history));
  }
  if (typeof api.dispatch === 'function') {
    for (const type of ['UPDATE_HISTORY_ENTITIES_REQUEST', 'GET_LOG_FILE_STATUSES_REQUEST']) {
      api.dispatch({ type, meta: { requestId: globalThis.crypto.randomUUID() } });
    }
  }
  return { entries, removed, kept };
}

function install({ api, ipcRenderer, models, refresh }) {
  if (!api || !ipcRenderer || !models) fail('install requires api, ipcRenderer and model constructors');
  if (installedApis.has(api)) return false;
  const required = ['Group','Host','Identity','SshKey','Snippet','SnippetPackage','PortForwardingRule','SshConfig','TelnetConfig','Proxy','HostChain'];
  for (const name of required) if (typeof models[name] !== 'function') fail(`missing model constructor ${name}`);
  if (typeof ipcRenderer.on !== 'function' || typeof ipcRenderer.send !== 'function') fail('invalid ipcRenderer');
  installedApis.add(api);
  let requestQueue = Promise.resolve();
  ipcRenderer.on('self-cloud:request', (event, request) => {
    if (event && typeof event.senderId === 'number' && event.senderId !== 0) return;
    requestQueue = requestQueue.then(async () => {
      const id = request && request.id;
      try {
        if (!request || !['export', 'import', 'clear-logs'].includes(request.operation)) fail('invalid request operation');
        if (request.operation === 'export') ipcRenderer.send('self-cloud:response', { id, ok: true, snapshot: await exportSnapshot(api) });
        else if (request.operation === 'clear-logs') ipcRenderer.send('self-cloud:response', { id, ok: true, result: await clearSessionLogs(api, request.since) });
        else ipcRenderer.send('self-cloud:response', { id, ok: true, result: await replaceSnapshot(api, request.snapshot, models, { refresh }) });
      } catch (error) {
        ipcRenderer.send('self-cloud:response', { id, ok: false, error: errorText(error) });
      }
    }).catch(() => {}); // an IPC send failure must not poison later requests
    return requestQueue;
  });
  ipcRenderer.send('self-cloud:register');
  return true;
}

module.exports = { FORMAT, VERSION, install, exportSnapshot, validateSnapshot, replaceSnapshot, clearSessionLogs, _private: { clearPortableData, writePortableData, detectUnsupported } };
