// Copy just the final Apple Silicon DMG to dist/ and discard temporary web assets.
import fs from 'node:fs';
import path from 'node:path';

if (process.platform !== 'darwin') throw new Error('macOS bundle publisher requires macOS.');
const root = path.resolve(import.meta.dirname, '..');
const version = process.env.TERMIAN_RELEASE_VERSION ||
  JSON.parse(fs.readFileSync(path.join(root, 'package.json'), 'utf8')).version;
const dist = path.join(root, 'dist');
const target = path.join(dist, '.cargo');
function findDmgs(dir) {
  if (!fs.existsSync(dir)) return [];
  const found = [];
  for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
    const pathname = path.join(dir, entry.name);
    if (entry.isDirectory()) found.push(...findDmgs(pathname));
    else if (entry.name.endsWith('.dmg')) found.push(pathname);
  }
  return found;
}
const files = findDmgs(target);
if (files.length !== 1) throw new Error('Expected one built macOS DMG, got: ' + JSON.stringify(files));
const destination = path.join(dist, 'termian-' + version + '-mac-arm64.dmg');
if (fs.existsSync(destination)) fs.rmSync(destination);
fs.renameSync(files[0], destination);
fs.rmSync(path.join(dist, 'web'), { recursive: true, force: true });
fs.rmSync(path.join(root, 'src', 'gen'), { recursive: true, force: true });
// Preserve compiled dependencies so the CI Rust cache can save them after
// publishing. The user can still request the old disk-saving cleanup.
if (process.env.TERMIAN_CLEAN_BUILD_CACHE === '1') {
  fs.rmSync(target, { recursive: true, force: true });
}
console.log('Single macOS release:', destination);
