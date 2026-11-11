import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { webcrypto } from 'node:crypto';
import {
  indexSnapshot, baselineOf, compareSnapshots, serverKey, saveBaseline, readBaseline, SNAPSHOT_FORMAT,
} from '../src/bridge/sync-diff.js';

if (!globalThis.crypto) globalThis.crypto = webcrypto;

const empty = () => ({
  format: SNAPSHOT_FORMAT, version: 1, createdAt: new Date().toISOString(),
  groups: [], hosts: [], identities: [], sshKeys: [], snippets: [], snippetPackages: [],
  portForwardingRules: [], knownHosts: [], sshConfigs: [], telnetConfigs: [],
  proxies: [], workspaceTemplates: [], settings: {},
});
const host = (name, addr, id) => ({
  local_id: id, label: name, address: addr, changedFields: [], status: 'new',
  encrypted_with: null, ssh_config: {local_id: 777, port: 22},
});

test('One-sided changes indicate exactly where the new data is', async () => {
  const common = empty();
  const a = await indexSnapshot(common);
  const baseline = baselineOf(a);
  const local = empty(); local.hosts.push(host('Web', '10.0.0.12', 50));
  const localDiff = compareSnapshots(await indexSnapshot(local), a, baseline);
  assert.equal(localDiff.kind, 'upload');
  assert.equal(localDiff.upload, 1);
  assert.match(localDiff.changes[0].text, /На устройстве: добавлен.*Хост «Web»/);
  const remoteDiff = compareSnapshots(a, await indexSnapshot(local), baseline);
  assert.equal(remoteDiff.kind, 'download');
  assert.equal(remoteDiff.download, 1);
  assert.match(remoteDiff.changes[0].text, /В облаке: добавлен.*Хост «Web»/);
});

test('The same record with regenerated importer IDs does not show a false change', async () => {
  const a = empty(); a.hosts.push(host('Production', 'example.test', 10));
  const b = empty(); b.hosts.push(host('Production', 'example.test', 9000));
  b.createdAt = '1980-01-01T00:00:00.000Z';
  b.hosts[0].ssh_config.local_id = 8293;
  b.hosts[0].changedFields = ['name'];
  const ai = await indexSnapshot(a), bi = await indexSnapshot(b);
  assert.equal(compareSnapshots(ai, bi, baselineOf(ai)).kind, 'synced');
});

test('Separate local and remote edits are conflicts, never direction guesses', async () => {
  const baselineSnapshot = empty(); baselineSnapshot.hosts.push(host('A', '192.0.2.1', 1));
  const baseline = baselineOf(await indexSnapshot(baselineSnapshot));
  const local = empty(); local.hosts.push(host('A', '192.0.2.2', 1));
  const remote = empty(); remote.hosts.push(host('A', '192.0.2.3', 1));
  const result = compareSnapshots(await indexSnapshot(local), await indexSnapshot(remote), baseline);
  assert.equal(result.kind, 'conflict');
  assert.equal(result.conflicts, 1);
  assert.match(result.changes[0].text, /Конфликт.*Хост «A»/);
  assert.match(result.changes[0].text, /Адрес: 192\.0\.2\.3 → 192\.0\.2\.2/);
});

test('Without a confirmed baseline, two differing snapshots have unknown direction', async () => {
  const local = empty(); local.snippets.push({local_id:1, label:'Reset', script:'echo local'});
  const remote = empty(); remote.snippets.push({local_id:2, label:'Reset', script:'echo remote'});
  const result = compareSnapshots(await indexSnapshot(local), await indexSnapshot(remote), null);
  assert.equal(result.kind, 'uncertain');
  assert.equal(result.unknown, 1);
  assert.equal(compareSnapshots(await indexSnapshot(local), await indexSnapshot(local), null).kind, 'synced');
});

test('Missing remote snapshot is never classified as safe to download after sync', async () => {
  const local = empty(); local.hosts.push(host('A', '10.10.1.1', 3));
  const indexed = await indexSnapshot(local);
  const state = compareSnapshots(indexed, new Map(), baselineOf(indexed), false);
  assert.equal(state.kind, 'conflict');
  assert.match(state.changes[0].text, /Снимок в облаке удалён/);
});

test('An empty snapshot and an absent cloud snapshot are not false changes', async () => {
  const local = await indexSnapshot(empty());
  assert.equal(local.size, 0);
  const state = compareSnapshots(local,new Map(),null,false);
  assert.equal(state.kind,'empty');
});

test('Only hashed baseline, never SSH passwords or private keys, enters localStorage', async () => {
  const local = empty(); local.sshKeys.push({local_id:3,label:'My Key',private_key:'SECRET PRIVATE KEY'});
  const indexed = await indexSnapshot(local);
  const cache = new Map();
  const storage = {getItem: key => cache.get(key),setItem:(key,value) => cache.set(key,value)};
  const key = await serverKey('https://example.test/v1/snapshot');
  saveBaseline(storage, key, indexed);
  const bytes = [...cache.values()].join('');
  assert.ok(!bytes.includes('SECRET'));
  assert.ok(!bytes.includes('My Key'));
  assert.ok(!bytes.includes('example.test'));
  assert.deepEqual(readBaseline(storage,key),{local: baselineOf(indexed), remote: baselineOf(indexed)});
  assert.equal(readBaseline(storage,await serverKey('https://another.test')),null);
});

test('Toolbar omits fake notification bell and vault Personal dropdown', () => {
  const ui = fs.readFileSync(new URL('../src/ui/assets/ui-process-62a480c2.js',import.meta.url),'utf8');
  const header = ui.slice(ui.indexOf('const Nqe = be.memo('), ui.indexOf('function jqe('));
  assert.ok(header.includes('p(eHc, {})'), 'Sync buttons remain mounted');
  assert.ok(!header.includes('p(Z$e, { themeColorsState: t12 })'), 'Bell removed from header');
  assert.ok(!header.includes('endAdornment: p(zF,'), 'Vault dropdown removed from header');
  assert.ok(header.includes('data-sync-direction'), 'Each sync button has an indicator');
  assert.ok(header.includes('data-sync-state'));
});

test('Sync counters are fully visible outside the 30px icon button', () => {
  const ui = fs.readFileSync(new URL('../src/ui/assets/ui-process-62a480c2.js', import.meta.url), 'utf8');
  const start = ui.indexOf('}), eHc = () => {');
  const end = ui.indexOf('}), Fqe = (', start);
  const toolbar = ui.slice(start, end);
  assert.ok(toolbar.includes('indicatorAnchor: { position: "relative"'));
  assert.ok(toolbar.includes('overflow: "visible"'), 'The anchor may not clip badges');
  assert.ok(toolbar.includes('display: "inline-flex", alignItems: "center", justifyContent: "center"'));
  assert.ok(toolbar.includes('height: count ? "16px" : "8px"'), 'Counters must fit their 16px line height');
  assert.ok(toolbar.includes('count > 99 ? "99+"'), 'Large counts should use a readable overflow label');
  assert.ok(toolbar.includes('children: e(true) }), indicator("upload")'), 'Upload badge belongs beside, not inside, the icon button');
  assert.ok(toolbar.includes('children: e(false) }), indicator("download")'), 'Download badge belongs beside, not inside, the icon button');
});

test('After a download, importer normalization is not offered for download again', async () => {
  // The cloud snapshot and the local export after importing it are
  // semantically the same data but not byte-identical (importer defaults).
  const remote = empty(); remote.hosts.push(host('Web', '10.0.0.12', 1));
  const imported = empty(); imported.hosts.push({...host('Web', '10.0.0.12', 77), backspace: 'default'});
  const remoteIndex = await indexSnapshot(remote), localIndex = await indexSnapshot(imported);
  const storage = new Map();
  const store = {getItem: key => storage.get(key) ?? null, setItem: (key, value) => storage.set(key, value)};
  const server = await serverKey('http://192.168.1.10:8080');
  saveBaseline(store, server, localIndex, remoteIndex);
  const baseline = readBaseline(store, server);
  assert.equal(compareSnapshots(localIndex, remoteIndex, baseline).kind, 'synced');

  const newer = empty(); newer.hosts.push(host('Web', '10.0.0.99', 1));
  const cloudEdit = compareSnapshots(localIndex, await indexSnapshot(newer), baseline);
  assert.equal(cloudEdit.kind, 'download');
  const edited = empty(); edited.hosts.push({...host('Web', '10.0.0.50', 77), backspace: 'default'});
  assert.equal(compareSnapshots(await indexSnapshot(edited), remoteIndex, baseline).kind, 'upload');
});

test('A single-sided baseline saved by earlier versions is still understood', async () => {
  const a = empty(); a.hosts.push(host('Web', '10.0.0.12', 1));
  const index = await indexSnapshot(a);
  const storage = new Map([['termian:self-cloud:baseline:v1',
    JSON.stringify({version: 1, server: 'k', hashes: baselineOf(index)})]]);
  const baseline = readBaseline({getItem: key => storage.get(key) ?? null}, 'k');
  assert.equal(compareSnapshots(index, index, baseline).kind, 'synced');
  const b = empty(); b.hosts.push(host('Web', '10.0.0.13', 1));
  assert.equal(compareSnapshots(index, await indexSnapshot(b), baseline).kind, 'download');
});
