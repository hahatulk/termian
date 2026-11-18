// Standard Tauri 2 CLI with a custom, single-source-directory layout.
// TAURI_APP_PATH and TAURI_FRONTEND_PATH are supported by the upstream CLI.
// All generated output (including Cargo cache) is kept under dist/.
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const root = path.resolve(import.meta.dirname, '..');
const env = {
  ...process.env,
  TAURI_APP_PATH: path.join(root, 'src'),
  TAURI_FRONTEND_PATH: root,
  CARGO_TARGET_DIR: path.join(root, 'dist', '.cargo'),
};
const cli = path.join(root, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
const result = spawnSync(process.execPath, [cli, ...process.argv.slice(2)], {
  cwd: root,
  env,
  stdio: 'inherit',
  windowsHide: false,
});
if (result.error) throw result.error;
process.exit(result.status ?? 1);
