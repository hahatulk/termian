import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { syncOnce, toSyncForm, mergeSnapshots } from '../src/bridge/sync-merge.js';

const adapter = createRequire(import.meta.url)('../src/self-cloud-adapter.cjs');
const validate = snapshot => ({...snapshot, ...adapter.validateSnapshot(snapshot)});
const COLLECTIONS = ['groups', 'hosts', 'identities', 'sshKeys', 'snippets', 'snippetPackages',
  'portForwardingRules', 'knownHosts', 'sshConfigs', 'telnetConfigs', 'proxies', 'workspaceTemplates'];
const clone = value => structuredClone(value);

function emptySnapshot() {
  return {format: 'termius-portable-snapshot', version: 1, createdAt: '2026-11-11T00:00:00.000Z',
    ...Object.fromEntries(COLLECTIONS.map(name => [name, []])), settings: {}};
}

// A device's vault: numeric IndexedDB IDs that change on every import, like
// the background Store behind self-cloud-adapter.cjs.
class Device {
  constructor(name, start) {
    this.name = name;
    this.next = start;
    this.data = emptySnapshot();
    this.saved = null;
    this.backups = 0;
  }
  id() { return this.next++; }
  addHost(label, { port = 22, group = null } = {}) {
    const config = {local_id: this.id(), port, identity: null, proxy: null, startup_snippet: null, host_chain: null};
    this.data.sshConfigs.push(config);
    this.data.hosts.push({local_id: this.id(), label, address: label + '.lan', group: group && {local_id: group.local_id},
      ssh_config: clone(config), telnet_config: null});
  }
  addGroup(label) {
    const group = {local_id: this.id(), label, parent_group: null, ssh_config: null, telnet_config: null};
    this.data.groups.push(group);
    return group;
  }
  host(label) { return this.data.hosts.find(host => host.label === label); }
  config(host) { return this.data.sshConfigs.find(config => config.local_id === host.ssh_config.local_id); }
  labels(kind = 'hosts') { return this.data[kind].map(record => record.label).sort(); }
  exportLocal = async () => validate(clone(this.data));
  importLocal = async snapshot => {
    // Every record gets a new local_id; report [snapshot id, new id] pairs.
    const pairs = {}, table = {};
    for (const kind of COLLECTIONS) {
      pairs[kind] = []; table[kind] = {};
      for (const record of snapshot[kind]) {
        const id = this.id();
        pairs[kind].push([record.local_id, id]);
        table[kind][typeof record.local_id + ':' + record.local_id] = id;
      }
    }
    const imported = toSyncForm(clone(snapshot), table);
    this.data = validate({...imported, knownHosts: []});
    return pairs;
  };
}

function server() {
  const state = {snapshot: null, revision: 0, uploads: 0};
  return {
    state,
    fetchRemote: async () => state.snapshot
      ? {exists: true, revision: 'r' + state.revision, snapshot: clone(state.snapshot)}
      : {exists: false, revision: null, snapshot: null},
    upload: async (snapshot, { revision, create }) => {
      if (create ? state.snapshot : revision !== 'r' + state.revision) {
        throw new Error('Снимок в облаке изменился во время синхронизации (HTTP 412)');
      }
      state.snapshot = clone(snapshot);
      state.revision++;
      state.uploads++;
    },
  };
}

async function sync(device, cloud, prefer = null) {
  const result = await syncOnce({
    saved: device.saved, prefer,
    exportLocal: device.exportLocal, importLocal: device.importLocal,
    fetchRemote: cloud.fetchRemote, upload: cloud.upload,
    backup: async () => { device.backups++; }, validate,
  });
  if (!result.conflicts.length) device.saved = result.state;
  return result;
}

test('Devices creating different hosts are merged, without duplicates', async () => {
  const cloud = server();
  const a = new Device('A', 1), b = new Device('B', 500);
  a.addHost('web');
  await sync(a, cloud);
  assert.equal(cloud.state.snapshot.hosts.length, 1);
  assert.equal(typeof cloud.state.snapshot.hosts[0].local_id, 'string', 'the cloud keeps stable sync IDs');

  await sync(b, cloud);
  assert.deepEqual(b.labels(), ['web']);

  a.addHost('api');
  b.addHost('db');
  const fromA = await sync(a, cloud);
  assert.equal(fromA.uploaded, true);
  const fromB = await sync(b, cloud);
  assert.deepEqual([fromB.fromLocal, fromB.fromRemote], [2, 2], 'db (+config) up, api (+config) down');
  await sync(a, cloud);
  assert.deepEqual(a.labels(), ['api', 'db', 'web']);
  assert.deepEqual(b.labels(), ['api', 'db', 'web']);
  assert.equal(cloud.state.snapshot.hosts.length, 3);
  assert.equal(cloud.state.snapshot.sshConfigs.length, 3);

  // Converged: another round changes nothing on either side.
  const uploads = cloud.state.uploads;
  for (const device of [a, b]) {
    const again = await sync(device, cloud);
    assert.deepEqual([again.uploaded, again.imported], [false, false], device.name);
  }
  assert.equal(cloud.state.uploads, uploads);
});

test('Edits of different records merge; the same record edited twice is a conflict', async () => {
  const cloud = server();
  const a = new Device('A', 1), b = new Device('B', 500);
  a.addHost('web'); a.addHost('db');
  await sync(a, cloud);
  await sync(b, cloud);

  // A changes web's port (its SSH config), B renames db: no conflict.
  a.config(a.host('web')).port = 2222;
  a.host('web').ssh_config.port = 2222;
  b.host('db').label = 'database';
  await sync(a, cloud);
  const merged = await sync(b, cloud);
  assert.deepEqual(merged.conflicts, []);
  assert.equal(b.config(b.host('web')).port, 2222);
  await sync(a, cloud);
  assert.deepEqual(a.labels(), ['database', 'web']);

  // Both rename web: stop and report, never pick silently.
  a.host('web').label = 'web-a';
  b.host('web').label = 'web-b';
  await sync(a, cloud);
  const conflict = await sync(b, cloud);
  assert.equal(conflict.conflicts.length, 1);
  assert.equal(conflict.conflicts[0].kind, 'hosts');
  assert.deepEqual(b.labels(), ['database', 'web-b'], 'nothing is applied on a conflict');
  const resolved = await sync(b, cloud, 'local');
  assert.deepEqual(resolved.conflicts, []);
  await sync(a, cloud);
  assert.deepEqual(a.labels(), ['database', 'web-b']);
});

test('Deletions propagate, and a record still referenced is kept', async () => {
  const cloud = server();
  const a = new Device('A', 1), b = new Device('B', 500);
  const group = a.addGroup('prod');
  a.addHost('web');
  a.addHost('old');
  await sync(a, cloud);
  await sync(b, cloud);

  a.data.hosts = a.data.hosts.filter(host => host.label !== 'old');
  // A deletes the group while B puts a new host into it.
  a.data.groups = [];
  b.addHost('db', {group: b.data.groups.find(item => item.label === 'prod')});
  await sync(a, cloud);
  const result = await sync(b, cloud);
  assert.deepEqual(result.conflicts, []);
  assert.deepEqual(b.labels(), ['db', 'web']);
  assert.deepEqual(b.labels('groups'), ['prod'], 'the group db needs survives the deletion');
  await sync(a, cloud);
  assert.deepEqual(a.labels(), ['db', 'web']);
  assert.deepEqual(a.labels('groups'), ['prod']);
  assert.ok(group);
});

test('First merge sync matches records both devices already have by name', async () => {
  const cloud = server();
  const a = new Device('A', 1), b = new Device('B', 500);
  a.addHost('web'); b.addHost('web');
  b.addHost('extra');
  // An older client uploaded A's data with plain numeric IDs.
  cloud.state.snapshot = validate(clone(a.data));
  const result = await sync(b, cloud);
  assert.deepEqual(result.conflicts, []);
  assert.deepEqual(cloud.state.snapshot.hosts.map(host => host.label).sort(), ['extra', 'web']);
  assert.equal(cloud.state.snapshot.sshConfigs.length, 2, 'the SSH config of web is matched through the host');

  // The same name with different content needs a decision.
  const c = new Device('C', 900);
  c.addHost('web', {port: 2200});
  const conflict = await sync(c, cloud);
  assert.equal(conflict.conflicts.length, 1);
  assert.equal(conflict.conflicts[0].label, 'web');
});

test('A snapshot changed during the sync is not overwritten', async () => {
  const cloud = server();
  const a = new Device('A', 1);
  a.addHost('web');
  await sync(a, cloud);
  a.addHost('api');
  const racing = {...cloud, upload: async (snapshot, options) => {
    cloud.state.revision++; // another device uploaded meanwhile
    return cloud.upload(snapshot, options);
  }};
  await assert.rejects(sync(a, racing), /изменился во время синхронизации/);
  assert.equal(cloud.state.snapshot.hosts.length, 1);
});

test('A deleted server snapshot is re-created instead of deleting local data', async () => {
  const cloud = server();
  const a = new Device('A', 1);
  a.addHost('web');
  await sync(a, cloud);
  cloud.state.snapshot = null;
  await sync(a, cloud);
  assert.deepEqual(a.labels(), ['web']);
  assert.equal(cloud.state.snapshot.hosts.length, 1);
});

test('Settings merge per key', async () => {
  const local = {...emptySnapshot(), settings: {font: 'A', theme: 'dark'}};
  const remote = {...emptySnapshot(), settings: {font: 'A', theme: 'light', size: 14}};
  const base = await mergeSnapshots({local, remote: local});
  assert.equal(base.uploadNeeded, false);
  const {digestsOf} = await import('../src/bridge/sync-merge.js');
  const merged = await mergeSnapshots({local: {...local, settings: {font: 'B', theme: 'dark'}}, remote,
    base: {local: await digestsOf(local), remote: await digestsOf(local)}});
  assert.deepEqual(merged.conflicts, []);
  assert.deepEqual(merged.local.settings, {font: 'B', theme: 'light', size: 14});
  assert.deepEqual(merged.remote.settings, {font: 'B', theme: 'light', size: 14});
});

test('Log cleanup deletes history entries of the period and reports the rest', async () => {
  const hour = 3_600_000, now = Date.now();
  const histories = [
    {local_id: 1, connected_at: now - 10 * 60_000, session_log_data: {name: 'recent.log'}},
    {local_id: 2, connected_at: new Date(now - 2 * hour).toISOString(), session_log_data: {name: 'older.log'}},
    {local_id: 3, connected_at: now - 5 * 60_000},
  ];
  const deleted = [], dispatched = [];
  const api = {
    store: {getHistoryEntities: async () => histories, deleteHistoryEntity: async history => { deleted.push(history.local_id); }},
    dispatch: action => dispatched.push(action.type),
  };
  assert.deepEqual(await adapter.clearSessionLogs(api, now - hour), {entries: 2, removed: ['recent.log'], kept: ['older.log']});
  assert.deepEqual(deleted, [1, 3]);
  assert.ok(dispatched.includes('UPDATE_HISTORY_ENTITIES_REQUEST'));
  deleted.length = 0;
  assert.equal((await adapter.clearSessionLogs(api, 0)).entries, 3, '"all" deletes every entry');
  await assert.rejects(adapter.clearSessionLogs(api, -1));
});

test('Header Sync button, hourly auto-sync switch and log cleanup are wired', async () => {
  const fs = await import('node:fs');
  const read = path => fs.readFileSync(new URL('../' + path, import.meta.url), 'utf8');
  const header = read('src/ui/assets/ui-process-62a480c2.js');
  assert.ok(header.includes('window.api.selfCloud.open("sync")'));
  assert.ok(header.includes('animateTransform'), 'the icon spins while syncing');
  assert.match(header, /children: syncOk \? "✓" : "!"/);
  const settings = read('src/ui/assets/index-919edcdd.js');
  assert.ok(settings.includes('r.setAutoSync(g.target.checked)'));
  assert.match(settings, /Hs_PERIODS = \[\["hour".+\["all", "Все", "все", 0\]\]/);
  const bridge = read('src/bridge/self-cloud.js');
  assert.ok(bridge.includes('const AUTO_SYNC_INTERVAL = 60 * 60_000;'));
  assert.ok(bridge.includes("metadata.autoSync === false"));
  const rust = read('src/rust/lib.rs');
  for (const command of ['native::self_cloud_fetch', 'native::self_cloud_set_auto_sync', 'session_logs::session_logs_clear']) {
    assert.ok(rust.includes(command), command);
  }
});
