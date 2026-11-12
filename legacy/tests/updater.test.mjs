import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { buildManifest } from '../scripts/updater-manifest.mjs';
import { installUpdateNotice, createUpdatesApi } from '../src/bridge/update-notice.js';

const base = path.resolve(import.meta.dirname, '..');
const read = relative => fs.readFileSync(path.resolve(base, relative), 'utf8');
const fakeSignature = version => Buffer.from(
  `untrusted comment: signature from tauri secret key\nRUR=\ntrusted comment: timestamp:1\tfile:x\tversion:${version}\nAAA=\n`).toString('base64');

function releaseDir(version, signedVersion = version) {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'termian-manifest-'));
  for (const asset of [`termian-${version}-mac-arm64.app.tar.gz`, `termian-${version}-win-x64.exe`, `termian-${version}-win-x64-setup.exe`]) {
    fs.writeFileSync(path.join(dir, asset), 'package');
    fs.writeFileSync(path.join(dir, asset + '.sig'), fakeSignature(signedVersion));
  }
  return dir;
}

test('latest.json points each platform at its signed release asset', () => {
  const dir = releaseDir('1.4.0');
  const manifest = buildManifest({dir, version: '1.4.0', tag: 'v1.4.0', repository: 'hahatulk/termius', notes: 'n', date: new Date(0)});
  assert.deepEqual(Object.keys(manifest.platforms).sort(), ['darwin-aarch64', 'windows-x86_64', 'windows-x86_64-nsis']);
  assert.equal(manifest.platforms['windows-x86_64-nsis'].url,
    'https://github.com/hahatulk/termius/releases/download/v1.4.0/termian-1.4.0-win-x64-setup.exe',
    'the installed app updates through its installer');
  assert.equal(manifest.version, '1.4.0');
  assert.equal(manifest.platforms['windows-x86_64'].url,
    'https://github.com/hahatulk/termius/releases/download/v1.4.0/termian-1.4.0-win-x64.exe');
  assert.equal(manifest.platforms['darwin-aarch64'].url,
    'https://github.com/hahatulk/termius/releases/download/v1.4.0/termian-1.4.0-mac-arm64.app.tar.gz');
  assert.equal(manifest.platforms['darwin-aarch64'].signature, fakeSignature('1.4.0'));
  fs.rmSync(dir, {recursive: true});
});

test('Manifest refuses unsigned, missing or version-mismatched packages', () => {
  const mismatched = releaseDir('1.4.0', '1.3.0');
  assert.throws(() => buildManifest({dir: mismatched, version: '1.4.0', tag: 'v1.4.0', repository: 'a/b'}), /not bound to version/);
  const unsigned = releaseDir('1.4.0');
  fs.rmSync(path.join(unsigned, 'termian-1.4.0-win-x64.exe.sig'));
  assert.throws(() => buildManifest({dir: unsigned, version: '1.4.0', tag: 'v1.4.0', repository: 'a/b'}), /Missing update signature/);
  assert.throws(() => buildManifest({dir: unsigned, version: '1.4.0', tag: 'v1.5.0', repository: 'a/b'}), /does not match/);
  for (const dir of [mismatched, unsigned]) fs.rmSync(dir, {recursive: true});
});

test('Updates are verified natively and cannot be driven from the webview', () => {
  const config = JSON.parse(read('src/tauri.conf.json'));
  const updater = config.plugins.updater;
  assert.match(Buffer.from(updater.pubkey, 'base64').toString('utf8'), /minisign public key/);
  assert.equal(updater.requireSignedVersion, true, 'Signatures must be bound to the announced version');
  const capability = JSON.parse(read('src/capabilities/default.json'));
  assert.ok(!capability.permissions.some(permission => permission.startsWith('updater:')), 'JS must not call the updater plugin directly');
  const lib = read('src/rust/lib.rs');
  assert.match(lib, /tauri_plugin_updater::Builder::new\(\)\.build\(\)/);
  assert.match(lib, /updater::start\(app\.handle\(\)\)/);
  assert.match(lib, /updater::update_status,\s*updater::update_check,\s*updater::update_restart/);
  const rust = read('src/rust/updater.rs');
  assert.match(rust, /option_env!\("TERMIAN_RELEASE_VERSION"\)\.is_some\(\)/, 'Local builds must not update themselves');
  assert.match(rust, /url\.host_str\(\) == Some\("github\.com"\)/);
});

test('Release workflow signs update packages and publishes latest.json', () => {
  const workflow = read('../.github/workflows/release.yml');
  assert.match(workflow, /TAURI_SIGNING_PRIVATE_KEY: \$\{\{ secrets\.TAURI_SIGNING_PRIVATE_KEY \}\}/);
  assert.match(workflow, /node scripts\/updater-artifacts\.mjs/);
  assert.match(workflow, /node legacy\/scripts\/updater-manifest\.mjs release-assets/);
  for (const asset of ['mac-arm64.app.tar.gz', 'mac-arm64.app.tar.gz.sig', 'win-x64.exe.sig', 'win-x64-setup.exe', 'win-x64-setup.exe.sig']) {
    assert.ok(workflow.includes('"termian-${RELEASE_VERSION}-' + asset + '"'), 'Release must require ' + asset);
  }
  assert.equal((workflow.match(/TERMIAN_UPDATE_REPOSITORY: \$\{\{ github\.repository \}\}/g) || []).length, 2);
  const artifacts = read('scripts/updater-artifacts.mjs');
  assert.match(artifacts, /'--app-version', version/);
  assert.match(artifacts, /'-C', bundle, 'termian\.app'/);
});

// Just enough DOM for the notice.
function fakeDocument() {
  const element = tag => ({
    tag, children: [], dataset: {}, attributes: {}, textContent: '', id: '',
    append(...nodes) { this.children.push(...nodes); },
    appendChild(node) { this.children.push(node); },
    setAttribute(name, value) { this.attributes[name] = value; },
    remove() { this.removed = true; },
  });
  const head = element('head'), body = element('body');
  return {head, body, createElement: element, getElementById: () => null};
}
const find = (node, test) => test(node) ? node : node.children?.map(child => find(child, test)).find(Boolean);

test('Installed update shows a restart notice; restart runs only on request', async () => {
  const doc = fakeDocument();
  const calls = [];
  let emit;
  const invoke = async command => { calls.push(command); return command === 'update_status' ? {state: 'idle'} : undefined; };
  installUpdateNotice({invoke, listen: async (_event, handler) => { emit = handler; }, doc});
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(doc.body.children.length, 0);
  emit({payload: {state: 'installed', version: '1.4.0'}});
  const notice = doc.body.children[0];
  assert.ok(find(notice, node => node.textContent === 'Termian 1.4.0 установлен'));
  assert.deepEqual(calls, ['update_status']);
  find(notice, node => node.textContent === 'Перезапустить').onclick();
  assert.deepEqual(calls, ['update_status', 'update_restart']);
  find(notice, node => node.textContent === 'Позже').onclick();
  emit({payload: {state: 'installed', version: '1.4.0'}});
  assert.equal(doc.body.children.length, 1, 'Dismissed notice stays hidden until the next launch');
});

test('Windows ships a portable EXE and a per-user NSIS installer from one binary', () => {
  const config = JSON.parse(read('src/tauri.conf.json'));
  assert.equal(config.bundle.windows.nsis.installMode, 'currentUser', 'no admin rights: the updater can replace it');
  assert.equal(config.plugins.updater.windows.installMode, 'passive');
  const publish = read('scripts/publish-windows.mjs');
  // The portable copy is taken before `tauri bundle` stamps the binary.
  assert.ok(publish.indexOf('fs.writeFileSync(destination, bytes)') < publish.indexOf("'bundle', '--bundles', 'nsis'"));
  assert.ok(publish.includes("__TAURI_BUNDLE_TYPE_VAR_UNK"));
  assert.ok(publish.includes("-win-x64-setup.exe"));
  const artifacts = read('scripts/updater-artifacts.mjs');
  assert.ok(artifacts.includes('`termian-${version}-win-x64-setup.exe`'));
  const lib = read('src/rust/lib.rs');
  assert.match(lib, /BundleType::Nsis/);
  assert.match(lib, /!installed_build\(\) && std::env::var_os\("WEBVIEW2_USER_DATA_FOLDER"\)/, 'only the portable EXE keeps WebView2 data next to itself');
  assert.match(lib, /updater::install_pending\(\);\s*window\.app_handle\(\)\.exit\(0\)/);
  const updater = read('src/rust/updater.rs');
  assert.match(updater, /if crate::installed_build\(\) \{/);
});

test('Settings → Version checks for updates and shows download progress', async () => {
  let emit;
  const calls = [];
  const invoke = async command => {
    calls.push(command);
    if (command === 'update_status') return {state: 'idle'};
    if (command === 'update_check') return {state: 'checking'};
  };
  const updates = createUpdatesApi({invoke, listen: async (_event, handler) => { emit = handler; }});
  await new Promise(resolve => setImmediate(resolve));
  const seen = [];
  updates.subscribe(status => seen.push(status.state));
  await updates.check();
  emit({payload: {state: 'downloading', version: '1.5.0', downloaded: 512, total: 1024}});
  emit({payload: {state: 'installed', version: '1.5.0'}});
  assert.deepEqual(seen, ['checking', 'downloading', 'installed']);
  assert.equal(updates.status().state, 'installed');
  await updates.restart();
  assert.deepEqual(calls, ['update_status', 'update_check', 'update_restart']);

  const page = read('src/ui/assets/index-f96c7727.js');
  assert.ok(page.includes('window.api.updates'));
  assert.ok(page.includes('"Проверить обновления"') && page.includes('"Перезапустить и обновить"'));
  assert.match(page, /\$\{percent\}% \(\$\{mb\(status\.downloaded \|\| 0\)\} из \$\{mb\(status\.total\)\} МБ\)/);
  const rust = read('src/rust/updater.rs');
  assert.match(rust, /pub fn update_check</);
  assert.match(rust, /app\.emit\(STATUS_EVENT, status\)/, 'status reaches the Settings window, not only main');
  assert.match(rust, /status\.downloaded = Some\(downloaded\)/);
  assert.ok(read('src/original-bridge.js').includes('updates: createUpdatesApi({invoke, listen})'));
});
