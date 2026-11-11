import fs from 'node:fs';
import path from 'node:path';
import { build } from 'esbuild';

// src/ui/ is the sole editable, reconstructed Termius frontend tree.
// Normal builds MUST NOT read the archived Electron bundles in ../app.
const tauriRoot = path.resolve(import.meta.dirname, '..');
const sources = path.join(tauriRoot, 'src/ui');
const out = path.join(tauriRoot, 'dist/web');
const assets = path.join(sources, 'assets');
const html = path.join(sources, 'index.html');
const version = process.env.TERMIAN_RELEASE_VERSION ||
  JSON.parse(fs.readFileSync(path.join(tauriRoot, 'package.json'), 'utf8')).version;

if (!fs.existsSync(html) || !fs.existsSync(assets)) {
  throw new Error('Missing UI sources in src/ui/. This repository keeps exactly one editable source tree; no archived copies are imported during build.');
}
const required = ['ui-process-62a480c2.js', 'reconnectSaga-3b11f62e.js', 'main-3bca6fa5.css',
  'setting-process-8b8e42f8.js', 'main-0debf4cf.js'];
for (const name of required) {
  if (!fs.existsSync(path.join(assets, name))) {
    throw new Error('Missing recovered source module: src/ui/assets/' + name);
  }
}
const markup = fs.readFileSync(html, 'utf8');
if (!markup.includes('id="root"') ||
    !markup.includes('./original-bridge.js') ||
    !markup.includes('./assets/ui-process-62a480c2.js')) {
  throw new Error('src/ui/index.html is missing required Termian bootstrap hooks.');
}

// Clean only the generated frontend. Never delete dist/ or its EXE, Cargo cache or user data.
fs.rmSync(out, { recursive: true, force: true });
fs.mkdirSync(out, { recursive: true });
fs.cpSync(assets, path.join(out, 'assets'), { recursive: true, force: true });
fs.copyFileSync(html, path.join(out, 'index.html'));
// Additional original renderers: the Settings window and the hidden background
// process that owns the vault database (see src/original-bridge.js).
for (const page of ['settings.html', 'background.html']) {
  fs.copyFileSync(path.join(sources, page), path.join(out, page));
}
await build({
  entryPoints: [path.join(tauriRoot, 'src/original-bridge.js')],
  bundle: true,
  platform: 'browser',
  format: 'iife',
  target: 'es2022',
  // Keep the synchronous preload bridge compact: it runs before React's first paint.
  minify: true,
  define: { __TERMIAN_APP_VERSION__: JSON.stringify(version) },
  loader: { '.css': 'text' },
  outfile: path.join(out, 'original-bridge.js'),
  logLevel: 'warning',
});
console.log('Staged editable Termian UI from src/ui/:',
  fs.readdirSync(path.join(out, 'assets')).length,
  'assets; no Electron bundle copy performed');
