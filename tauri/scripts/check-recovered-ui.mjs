// Validate the editable recovered web sources before compiling another Tauri EXE.
import fs from 'node:fs';
import path from 'node:path';
import { transform } from 'esbuild';

const root = path.resolve(import.meta.dirname, '../src/ui');
const assetRoot = path.join(root, 'assets');
const names = fs.readdirSync(assetRoot);
let modules = 0;
let stylesheets = 0;

for (const name of names) {
  if (!name.endsWith('.js') && !name.endsWith('.css')) continue;
  const js = name.endsWith('.js');
  const source = fs.readFileSync(path.join(assetRoot, name), 'utf8');
  try {
    await transform(source, {
      loader: js ? 'js' : 'css',
      format: js ? 'esm' : undefined,
      target: 'esnext',
      minify: false,
      sourcemap: false,
      logLevel: 'silent',
    });
  } catch (error) {
    throw new Error('Syntax error in editable UI source ' + name + ': ' + error.message, { cause: error });
  }
  if (js) modules++; else stylesheets++;
}

const html = fs.readFileSync(path.join(root, 'index.html'), 'utf8');
for (const match of html.matchAll(/(?:src|href)="\.\/assets\/([^"]+)"/g)) {
  const name = decodeURIComponent(match[1]);
  if (!fs.existsSync(path.join(assetRoot, name))) {
    throw new Error('Missing asset referenced by src/ui/index.html: ' + name);
  }
}
console.log('Validated editable recovered UI:', modules, 'ES modules and', stylesheets, 'stylesheets');
