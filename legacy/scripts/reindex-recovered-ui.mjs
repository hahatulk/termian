// Refresh the searchable symbol index after editing readable recovered chunks.
import fs from 'node:fs';
import path from 'node:path';

const root = path.resolve(import.meta.dirname, '../src/ui');
const assets = path.join(root, 'assets');
const symbols = [];
for (const filename of fs.readdirSync(assets).sort()) {
  if (!filename.endsWith('.js')) continue;
  const code = fs.readFileSync(path.join(assets, filename), 'utf8');
  const pattern = /^(?:(?:export|declare)\s+)?(?:(?:async)\s+)?(function\s*\*?|class|(?:const|let|var))\s+([A-Za-z_$][\w$]*)/gm;
  let match;
  let last = 0;
  let line = 1;
  while ((match = pattern.exec(code))) {
    if (match[2].length < 5) continue;
    line += code.slice(last, match.index).split('\n').length - 1;
    last = match.index;
    symbols.push({ name: match[2], kind: match[1].trim(), module: filename, line });
  }
}
symbols.sort((a, b) => a.name.localeCompare(b.name) || a.module.localeCompare(b.module) || a.line - b.line);
fs.writeFileSync(path.join(root, 'symbols.json'), JSON.stringify(symbols, null, 2) + '\n', 'utf8');
console.log('Updated readable UI symbols:', symbols.length);
