import {test} from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';

const root=new URL('../',import.meta.url);
const read=relative=>fs.readFileSync(new URL(relative,root),'utf8');

test('Keychain lookups and writes do not block the Tauri main thread',()=>{
  const native=read('src/rust/native.rs');
  for(const name of ['get','set','delete','find']) {
    assert.match(native,new RegExp('\\#\\[tauri::command\\(async\\)\\]\\s+pub fn keychain_'+name+'\\('),
      'Synchronous Keychain calls may block the main UI while macOS asks for permission');
  }
});
test('Critical desktop boot bridge is minified before Tauri distribution',()=>{
  const staging=read('scripts/stage-original-ui.mjs');
  assert.match(staging,/entryPoints:\s*\[path\.join\(tauriRoot, 'src\/original-bridge\.js'\)\][\s\S]*?minify:\s*true/);
});
