// Signed auto-update packages for a release build (see src/rust/updater.rs).
// Windows: the portable EXE and the NSIS setup.exe themselves (the installed
// app updates through the installer). macOS: the signed .app as .app.tar.gz,
// the archive tauri-plugin-updater swaps in. Each gets a minisign .sig bound
// to the release version; the private key comes from TAURI_SIGNING_PRIVATE_KEY.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..');
const dist = path.join(root, 'dist');
const version = process.env.TERMIAN_RELEASE_VERSION;
if (!version || !/^\d+\.\d+\.\d+$/.test(version)) throw new Error('TERMIAN_RELEASE_VERSION must be a release SemVer');
if (!process.env.TAURI_SIGNING_PRIVATE_KEY) throw new Error('TAURI_SIGNING_PRIVATE_KEY is required to sign update packages');

const run = (command, args, options = {}) => {
  const result = spawnSync(command, args, {cwd: root, stdio: 'inherit', ...options});
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(command + ' ' + args.join(' ') + ' failed with ' + result.status);
};

let packages;
if (process.platform === 'win32') {
  packages = [`termian-${version}-win-x64.exe`, `termian-${version}-win-x64-setup.exe`].map(name => path.join(dist, name));
} else if (process.platform === 'darwin') {
  const bundle = path.join(dist, '.cargo', 'aarch64-apple-darwin', 'release', 'bundle', 'macos');
  if (!fs.existsSync(path.join(bundle, 'termian.app'))) throw new Error('Missing signed termian.app in ' + bundle);
  const archive = path.join(dist, `termian-${version}-mac-arm64.app.tar.gz`);
  fs.rmSync(archive, {force: true});
  // The updater drops the first path component: the archive must hold termian.app/.
  run('tar', ['-czf', archive, '-C', bundle, 'termian.app'], {env: {...process.env, COPYFILE_DISABLE: '1'}});
  packages = [archive];
} else {
  throw new Error('Update packages are built only on Windows and macOS');
}
for (const packagePath of packages) {
  if (!fs.existsSync(packagePath) || fs.statSync(packagePath).size < 1_000_000) throw new Error('Update package is missing or too small: ' + packagePath);
  fs.rmSync(packagePath + '.sig', {force: true});
  run(process.execPath, [path.join(root, 'scripts', 'tauri-cli.mjs'), 'signer', 'sign', '--app-version', version, packagePath],
    {env: {CI: 'true', TAURI_SIGNING_PRIVATE_KEY_PASSWORD: '', ...process.env}});
  if (!fs.existsSync(packagePath + '.sig')) throw new Error('Signature was not written: ' + packagePath + '.sig');
  console.log('Signed update package:', packagePath);
}
