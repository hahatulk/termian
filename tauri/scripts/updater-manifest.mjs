// Writes latest.json, the manifest tauri-plugin-updater reads, from the
// signed release assets. Usage:
//   node updater-manifest.mjs <assets-dir> <version> <tag> <owner/repo> [notes]
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const PLATFORMS = Object.freeze({
  'darwin-aarch64': version => `termian-${version}-mac-arm64.app.tar.gz`,
  // Portable EXE (no bundle type) and the NSIS-installed app: the plugin
  // looks up `{os}-{arch}-{bundle type}` before `{os}-{arch}`.
  'windows-x86_64': version => `termian-${version}-win-x64.exe`,
  'windows-x86_64-nsis': version => `termian-${version}-win-x64-setup.exe`,
});

export function buildManifest({ dir, version, tag, repository, notes = '', date = new Date() }) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error('Version must be a release SemVer: ' + version);
  if (tag !== 'v' + version) throw new Error(`Tag ${tag} does not match version ${version}`);
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository)) throw new Error('Repository must be owner/name: ' + repository);
  const platforms = {};
  for (const [target, assetName] of Object.entries(PLATFORMS)) {
    const asset = assetName(version);
    const signaturePath = path.join(dir, asset + '.sig');
    if (!fs.existsSync(path.join(dir, asset))) throw new Error('Missing update package: ' + asset);
    if (!fs.existsSync(signaturePath)) throw new Error('Missing update signature: ' + asset + '.sig');
    const signature = fs.readFileSync(signaturePath, 'utf8').trim();
    const comments = Buffer.from(signature, 'base64').toString('utf8');
    if (!comments.includes('\tversion:' + version + '\n')) throw new Error(asset + '.sig is not bound to version ' + version);
    platforms[target] = {
      signature,
      url: `https://github.com/${repository}/releases/download/${tag}/${asset}`,
    };
  }
  return { version, notes, pub_date: date.toISOString(), platforms };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [dir, version, tag, repository, notes = ''] = process.argv.slice(2);
  const manifest = buildManifest({ dir, version, tag, repository, notes });
  fs.writeFileSync(path.join(dir, 'latest.json'), JSON.stringify(manifest, null, 2) + '\n');
  console.log('Wrote update manifest for', Object.keys(manifest.platforms).join(', '));
}
