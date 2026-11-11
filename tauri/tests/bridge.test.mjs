import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createReduxRelay } from '../src/bridge/redux-relay.js';
import { crypto, errorCodes } from '../src/bridge/ssh-compat-crypto.js';
import { createKeyHelpers } from '../src/bridge/ssh-compat-keys.js';

// Throwaway keys generated for these tests only (never used for a server).
const keys = JSON.parse(fs.readFileSync(new URL('./fixtures-keys.json', import.meta.url), 'utf8'));

function fakeStore() {
  const actions = [];
  return {actions, dispatch(action) { actions.push(action); return action; }};
}

test('Redux relay reduces locally and fans actions out to other windows once', async () => {
  const ui = createReduxRelay({source: 'ui:3'});
  const background = createReduxRelay({source: 'background:1'});
  const uiStore = fakeStore();
  const backgroundStore = fakeStore();
  ui.replayActionRenderer(uiStore);
  background.replayActionRenderer(backgroundStore);

  const reduced = [];
  const dispatchFromUi = ui.forwardToMain(uiStore)(action => { reduced.push(action); return action; });
  dispatchFromUi({type: 'SAVE_KNOWN_HOST_REQUEST', payload: {hostnames: '[h]:2222'}});
  dispatchFromUi({type: '@@redux/INIT'});
  dispatchFromUi({type: 'LOCAL_ONLY', meta: {scope: 'local'}});
  await new Promise(resolve => setTimeout(resolve, 50));

  assert.deepEqual(reduced.map(action => action.type), ['SAVE_KNOWN_HOST_REQUEST', '@@redux/INIT', 'LOCAL_ONLY']);
  assert.equal(reduced[0].meta.scope, 'local', 'replayed actions must not be forwarded again');
  assert.deepEqual(backgroundStore.actions.map(action => action.type), ['SAVE_KNOWN_HOST_REQUEST']);
  assert.equal(uiStore.actions.length, 0, 'a window never receives its own broadcast');
  ui.close();
  background.close();
});

test('Redux relay keeps Map state when an action carries non-cloneable values', async () => {
  // The background broadcasts lifecycle/REHYDRATE with the whole persisted
  // state; sidepanel.windows is a Map that the UI selectors call .get() on.
  const background = createReduxRelay({source: 'background:1'});
  const ui = createReduxRelay({source: 'ui:3'});
  const uiStore = fakeStore();
  ui.replayActionRenderer(uiStore);
  const dispatch = background.forwardToMain(fakeStore())(action => action);
  const windows = new Map([[3, {visible: false}]]);
  dispatch({type: 'lifecycle/REHYDRATE', payload: {
    sidepanel: {visible: true, windows},
    sorting: {selected: new Set(['a'])},
    stamp: new Date(0),
    callback() {},
    nested: [{format: () => 'x', bytes: new Uint8Array([1, 2])}],
  }});
  await new Promise(resolve => setTimeout(resolve, 50));
  ui.close();
  background.close();

  const [received] = uiStore.actions;
  assert.equal(received.type, 'lifecycle/REHYDRATE');
  assert.ok(received.payload.sidepanel.windows instanceof Map, 'Map must not degrade to {}');
  assert.deepEqual(received.payload.sidepanel.windows.get(3), {visible: false});
  assert.ok(received.payload.sorting.selected instanceof Set);
  assert.ok(received.payload.stamp instanceof Date);
  assert.equal('callback' in received.payload, false);
  assert.equal('format' in received.payload.nested[0], false);
  assert.deepEqual([...received.payload.nested[0].bytes], [1, 2]);
});

test('Local vault crypto round-trips in the compatible v4 envelope and rejects tampering', () => {
  const key = crypto.utils.generateEncryptionKey();
  const system = crypto.systems.FromEncryptionKey(key);
  const sealed = system.encrypt(Buffer.from('secret password'));
  assert.equal(sealed[0], 4);
  assert.equal(sealed[1], 1);
  assert.match(sealed.toString('base64'), /^B/, 'the bundle detects v4 data by its base64 prefix');
  assert.equal(system.decrypt(sealed).toString(), 'secret password');
  sealed[sealed.length - 1] ^= 1;
  assert.equal(system.decrypt(sealed), null);
  assert.equal(system.getLastError(), errorCodes['crypto::Error::DecryptionFailed']);
  const other = crypto.systems.FromEncryptionKey(crypto.utils.generateEncryptionKey());
  assert.equal(other.decrypt(system.encrypt(Buffer.from('x'))), null);
});

test('Key helpers read type, encryption and public key from OpenSSH and PEM keys', async () => {
  const calls = [];
  const helpers = createKeyHelpers(async (command, args) => { calls.push({command, args}); return {privateKey: 'p', publicKey: 'q'}; });
  assert.equal(helpers.getKeyTypeFromPrivateKey(keys.ed), 'ed');
  assert.equal(helpers.checkPrivateKeyEncrypted(keys.ed), true);
  assert.equal(helpers.validatePrivateKeyFormat(keys.ed), true);
  assert.equal(helpers.generatePublicKeyFromPrivateKey(keys.ed), keys.edPub.split(' ').slice(0, 2).join(' '));
  assert.equal(helpers.getKeyTypeFromPrivateKey(keys.ec), 'ec');
  assert.equal(helpers.checkPrivateKeyEncrypted(keys.ec), false);
  assert.equal(helpers.getKeyTypeFromPrivateKey('-----BEGIN RSA PRIVATE KEY-----\nAAAA\n-----END RSA PRIVATE KEY-----'), 'rsa');
  assert.equal(helpers.validatePrivateKeyFormat('not a key'), false);
  assert.equal(helpers.getKeyTypeFromPrivateKey('not a key'), 'unknown');

  assert.equal(helpers.validatePublicKeyFormat(keys.edPub), true);
  assert.equal(helpers.validatePublicKeyFormat('ssh-ed25519 AAAAB3NzaC1yc2E='), false, 'blob type must match');
  assert.equal(helpers.checkPublicKeyIsCertificate(keys.ecPub), false);
  assert.equal(helpers.generateSshKeyFromPublicKey(keys.edPub).publicKey, keys.edPub.split(' ').slice(0, 2).join(' '));

  await helpers.generateSshKey({type: 'ec', curve: 'nistp521', password: '', comment: 'x'});
  assert.deepEqual(calls, [{command: 'ssh_generate_key', args: {options: {type: 'ec', bits: null, curve: 'nistp521', password: '', comment: 'x'}}}]);
});
