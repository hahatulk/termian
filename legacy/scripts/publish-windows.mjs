// Publishes both Windows releases from one compiled binary:
//   dist/termian-<version>-win-x64.exe        portable (profile in termian-data/)
//   dist/termian-<version>-win-x64-setup.exe  NSIS installer (per-user)
// The portable copy is taken BEFORE bundling: `tauri bundle` stamps the
// bundle type ("nsis") into the binary, and a stamped portable EXE would look
// installed to src/rust (app data profile, installer updates).
// Keep Cargo intermediates for faster incremental builds by default.
// Disk-sensitive clean builds can opt in with TERMIAN_CLEAN_BUILD_CACHE=1.
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

if (process.platform !== 'win32') {
  throw new Error('The Windows portable publisher must run on Windows.');
}
const root = path.resolve(import.meta.dirname, '..');
const version = process.env.TERMIAN_RELEASE_VERSION ||
  JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).version;
const built = path.join(root, 'dist', '.cargo', 'release', 'termian.exe');
const destinationDir = path.join(root, 'dist');
const destination = path.join(destinationDir, 'termian-' + version + '-win-x64.exe');
const installer = path.join(destinationDir, 'termian-' + version + '-win-x64-setup.exe');
const nsisDir = path.join(root, 'dist', '.cargo', 'release', 'bundle', 'nsis');
const releaseConfig = path.join(root, 'src', 'tauri.release.conf.json');
const stalePrefix = path.basename(destination) + '.replaced';
const oldCopy = destination + '.replaced-' + process.pid;

if (!fs.existsSync(built) || fs.statSync(built).size < 1_000_000) {
  throw new Error('Tauri release EXE is missing or unexpectedly small: ' + built);
}
// Previous copies can remain locked while an older application is running.
// Remove only unlocked backups; never replace or terminate a running binary.
for (const name of fs.readdirSync(destinationDir, {withFileTypes: true})) {
  if (!name.isFile() || !(name.name === stalePrefix || name.name.startsWith(stalePrefix + '-'))) continue;
  const stale = path.join(destinationDir, name.name);
  try { fs.rmSync(stale); }
  catch (error) {
    if (!['EPERM', 'EBUSY', 'EACCES'].includes(error.code)) throw error;
    console.warn('Old executable still in use; preserving:', stale);
  }
}

fs.mkdirSync(destinationDir, { recursive: true });
let hadPrevious = false;
try {
  if (fs.existsSync(destination)) {
    if (fs.existsSync(oldCopy)) throw new Error('Temporary backup name is already in use: ' + oldCopy);
    fs.renameSync(destination, oldCopy);
    hadPrevious = true;
  }
  // Copy: the bundler below still needs target/release/termian.exe.
  const bytes = fs.readFileSync(built);
  // An unstamped binary still holds the placeholder value (the NSIS name
  // also appears in tauri's own match arms, so test the placeholder).
  if (!bytes.includes(Buffer.from('__TAURI_BUNDLE_TYPE_VAR_UNK'))) {
    throw new Error('The compiled EXE is already stamped with a bundle type; rebuild before publishing the portable copy.');
  }
  fs.writeFileSync(destination, bytes);
  if (hadPrevious) {
    try { fs.rmSync(oldCopy); }
    catch (error) {
      // Windows can keep a previous EXE locked (running copy or antivirus).
      // The NEW EXE has already been atomically published; don't fail the
      // release just because the previous binary cannot be removed yet.
      if (error.code !== 'EPERM' && error.code !== 'EBUSY' && error.code !== 'EACCES') throw error;
      console.warn('Previous executable is locked; kept backup for later cleanup:', oldCopy);
    }
  }

  // Cargo may also leave an identically named linker output in release/deps.
  const redundant = path.join(root, 'dist', '.cargo', 'release', 'deps', 'termian.exe');
  if (fs.existsSync(redundant)) {
    fs.rmSync(redundant);
  }
} catch (error) {
  if (hadPrevious && !fs.existsSync(destination) && fs.existsSync(oldCopy)) {
    fs.renameSync(oldCopy, destination);
  }
  throw error;
}
// NSIS installer from the same binary (release version via the CI config).
fs.rmSync(nsisDir, { recursive: true, force: true });
const bundleArgs = ['bundle', '--bundles', 'nsis'];
if (fs.existsSync(releaseConfig)) bundleArgs.push('--config', releaseConfig);
const bundled = spawnSync(process.execPath, [path.join(root, 'scripts', 'tauri-cli.mjs'), ...bundleArgs], { cwd: root, stdio: 'inherit' });
if (bundled.error) throw bundled.error;
if (bundled.status !== 0) throw new Error('NSIS bundling failed with ' + bundled.status);
const setups = fs.existsSync(nsisDir) ? fs.readdirSync(nsisDir).filter(name => name.endsWith('-setup.exe')) : [];
if (setups.length !== 1) throw new Error('Expected one NSIS setup in ' + nsisDir + ', found: ' + setups.join(', '));
fs.rmSync(installer, { force: true });
fs.renameSync(path.join(nsisDir, setups[0]), installer);
if (fs.statSync(installer).size < 1_000_000) throw new Error('NSIS installer is unexpectedly small: ' + installer);

// The HTML/CSS/JS are now embedded in the EXE; keep just their one editable copy under src/.
// Recreated automatically by beforeBuildCommand or `npm run build:web` when needed.
fs.rmSync(path.join(root, 'dist', 'web'), { recursive: true, force: true });
// Tauri generates validation schemas while compiling; they are not editable sources.
fs.rmSync(path.join(root, 'src', 'gen'), { recursive: true, force: true });
// Preserve Cargo objects for the next local/CI build (the Rust cache action
// runs after this script). Never touch dist/termian-data or finished releases.
if (process.env.TERMIAN_CLEAN_BUILD_CACHE === '1') {
  fs.rmSync(path.join(root, 'dist', '.cargo'), { recursive: true, force: true });
  console.log('Opt-in Cargo cleanup completed (next Rust build will be cold).');
} else {
  console.log('Retained dist/.cargo/ for subsequent incremental Rust builds.');
}
console.log('Portable Windows release:', destination);
console.log('Windows installer:', installer);
console.log('Removed generated dist/web/; user profile and release are preserved.');
console.log('The dist/termian-data directory, if present, is untouched.');
