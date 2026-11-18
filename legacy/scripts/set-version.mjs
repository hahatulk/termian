import fs from 'node:fs';
import path from 'node:path';
const version = process.argv[2];
if (!version || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/.test(version)) {
  console.error('Expected numeric SemVer, e.g. 1.0.0');
  process.exit(2);
}
const base = path.resolve(import.meta.dirname, '..');
const tauriFile = path.join(base, 'src', 'tauri.release.conf.json');
fs.writeFileSync(tauriFile, JSON.stringify({version}, null, 2) + '\n');
console.log('Tauri release version:', version);
