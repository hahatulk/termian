import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { COLLECTIONS, indexSnapshot } from '../src/bridge/sync-diff.js';

const require = createRequire(import.meta.url);
const adapter = require('../src/self-cloud-adapter.cjs');

// Minimal stand-in for the recovered background Store: one host, one known host.
function fakeApi(settings) {
  let nextId = 100;
  const data = {
    hosts: [{local_id: 1, label: 'web', address: '10.0.0.1', group: null, ssh_config: null, telnet_config: null}],
    knownHosts: [{local_id: 7, hostname: 'web', public_key: 'ssh-ed25519 AAAA'}],
  };
  const deleted = [];
  const assign = list => value => { value.local_id = nextId++; data[list] ??= []; data[list].push(value); };
  const store = {
    collections: {known_hosts: data.knownHosts},
    getGroups: () => [], getHosts: () => data.hosts, getAllIdentities: () => [], getSshKeys: () => [],
    getSnippets: () => [], getSnippetsPackages: () => [], getPortForwardingRules: () => [],
    getKnownHosts: () => data.knownHosts, getSshConfigs: () => [], getTelnetConfigs: () => [],
    getProxies: () => [], getAllSepKeys: () => [], getAllTeamMultiKey: () => [],
    workspaceTemplatesRepository: {getMappedWorkspaceTemplates: () => [], save: async () => {}, delete: async () => {}},
    deleteHosts: async hosts => { data.hosts = data.hosts.filter(host => !hosts.includes(host)); },
    deleteKnownHost: async value => { deleted.push(value); },
    setKnownHost: async () => { throw new Error('known hosts must not be imported'); },
    setHost: assign('hosts'),
  };
  const model = {values: {...settings}, toJSON() { return {...this.values}; }, unset(key) { delete this.values[key]; }};
  const api = {store, settings: {getSettings: () => model, setSettings: async values => { model.values = {...values}; }}};
  return {api, data, deleted, model};
}
const models = new Proxy({}, {get: () => class { constructor(raw) { Object.assign(this, raw); } }});

test('Known hosts and log settings never enter an exported snapshot', async () => {
  const {api} = fakeApi({theme: 'dark', session_logs_enabled: true, logging: 'on', history_size: 5});
  const snapshot = await adapter.exportSnapshot(api);
  assert.deepEqual(snapshot.knownHosts, [], 'Field stays for format compatibility but is always empty');
  assert.deepEqual(snapshot.settings, {theme: 'dark'});
  assert.ok(snapshot.scope.excluded.includes('known hosts'));
  assert.ok(!snapshot.scope.included.includes('known hosts'));
  assert.equal(snapshot.hosts.length, 1);
});

test('Importing a snapshot keeps local known hosts and log settings, ignoring older cloud copies', async () => {
  const {api, data, deleted, model} = fakeApi({theme: 'dark', session_logs_enabled: false});
  const cloud = {
    format: adapter.FORMAT, version: adapter.VERSION,
    groups: [], hosts: [{local_id: 5, label: 'db', address: '10.0.0.2', group: null, ssh_config: null, telnet_config: null}],
    identities: [], sshKeys: [], snippets: [], snippetPackages: [], portForwardingRules: [],
    knownHosts: [{local_id: 9, hostname: 'db', public_key: 'ssh-rsa BBBB'}],
    sshConfigs: [], telnetConfigs: [], proxies: [], workspaceTemplates: [],
    settings: {theme: 'light', session_logs_enabled: true},
  };
  assert.deepEqual(adapter.validateSnapshot(cloud).knownHosts, []);
  await adapter.replaceSnapshot(api, cloud, models, {rollback: false});
  assert.deepEqual(deleted, [], 'Local known hosts must not be deleted');
  assert.equal(data.knownHosts.length, 1);
  assert.deepEqual(data.hosts.map(host => host.label), ['db']);
  assert.deepEqual(model.values, {theme: 'light', session_logs_enabled: false});
});

test('Sync comparison ignores known hosts from older cloud snapshots', async () => {
  assert.ok(!('knownHosts' in COLLECTIONS));
  const base = {format: 'termius-portable-snapshot', version: 1, groups: [], hosts: [], identities: [], sshKeys: [],
    snippets: [], snippetPackages: [], portForwardingRules: [], sshConfigs: [], telnetConfigs: [], proxies: [],
    workspaceTemplates: [], settings: {}};
  const index = await indexSnapshot({...base, knownHosts: [{local_id: 1, hostname: 'web'}]});
  assert.equal(index.size, 0);
});
