// Optional disk-space cleanup; never remove dist/ itself or the user profile.
import fs from 'node:fs';
import path from 'node:path';

const dir = path.resolve(import.meta.dirname, '..', 'dist', '.cargo');
fs.rmSync(dir, { recursive: true, force: true });
console.log('Removed only Cargo intermediates:', dir);
console.log('The next Rust build will recompile dependencies from scratch.');
