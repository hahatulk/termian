import { test } from 'node:test';
import assert from 'node:assert/strict';
import { promisify } from 'node:util';
import { createLocalFs } from '../src/bridge/local-fs.js';

// In-memory stand-in for src/rust/local_fs.rs (same commands and error shape).
function fakeNative() {
  const files = new Map([['/home/me/a.txt', Buffer.from('0123456789')]]);
  const dirs = new Set(['/', '/home', '/home/me']);
  const calls = [];
  const fail = (code, path) => Promise.reject({code, message: code + ': ' + path});
  const invoke = async (command, args) => {
    calls.push([command, args.op ?? null, args.path]);
    if (command === 'local_fs_read') {
      const data = files.get(args.path);
      if (!data) return fail('ENOENT', args.path);
      const slice = data.subarray(args.offset, args.offset + args.length);
      return slice.buffer.slice(slice.byteOffset, slice.byteOffset + slice.length);
    }
    if (command === 'local_fs_write') {
      const data = files.get(args.path);
      if (!data) return fail('ENOENT', args.path);
      const bytes = Buffer.from(args.data, 'base64');
      assert.equal(args.offset, data.length, 'chunks are written in order');
      files.set(args.path, Buffer.concat([data, bytes]));
      return null;
    }
    const { op, path } = args;
    if (path.startsWith('/protected')) return fail('EPERM', path);
    switch (op) {
      case 'realpath': return dirs.has(path) || files.has(path) ? path : fail('ENOENT', path);
      case 'readdir': return [...files.keys(), ...dirs].filter(p => p !== path && p.startsWith(path + '/') && !p.slice(path.length + 1).includes('/')).map(p => p.slice(path.length + 1));
      case 'lstat': return dirs.has(path) ? {mode: 0o40755, uid: 1, gid: 2, size: 0, atimeMs: 1000, mtimeMs: 2000, isDirectory: true, isFile: false, isSymbolicLink: false}
        : files.has(path) ? {mode: 0o100644, uid: 1, gid: 2, size: files.get(path).length, atimeMs: 1000, mtimeMs: 2000, isDirectory: false, isFile: true, isSymbolicLink: false}
        : fail('ENOENT', path);
      case 'mkdir': if (dirs.has(path)) return fail('EEXIST', path); dirs.add(path); return null;
      case 'truncate': files.set(path, Buffer.alloc(0)); return null;
      default: return fail('EINVAL', path);
    }
  };
  return { invoke, files, dirs, calls };
}

test('Local fs exposes Node callbacks that the UI promisifies', async () => {
  const native = fakeNative();
  const fs = createLocalFs({invoke: native.invoke});
  assert.equal(await promisify(fs.realpath)('/home/me'), '/home/me');
  assert.deepEqual((await promisify(fs.readdir)('/home/me')).sort(), ['a.txt']);
  const stat = await promisify(fs.lstat)('/home/me/a.txt');
  assert.equal(stat.isDirectory(), false);
  assert.equal(stat.isSymbolicLink(), false);
  assert.equal(stat.size, 10);
  assert.equal(stat.mtime.getTime(), 2000, 'ui-process reads atime/mtime as Date');
  await assert.rejects(promisify(fs.lstat)('/home/me/missing'), error => error.code === 'ENOENT');
  await promisify(fs.mkdir)('/home/me/x/y', {recursive: true});
  assert.ok(native.dirs.has('/home/me/x/y'));
});

test('Permission errors keep their code and open the macOS access guide', async () => {
  let denied = 0;
  const fs = createLocalFs({invoke: fakeNative().invoke, onPermissionDenied: () => denied++});
  await assert.rejects(promisify(fs.readdir)('/protected/Documents'), error => error.code === 'EPERM' && error.path === '/protected/Documents');
  assert.equal(denied, 1);
  // Listing attributes of protected entries is silently ignored by the UI.
  await assert.rejects(promisify(fs.lstat)('/protected/Mail'), error => error.code === 'EPERM');
  assert.equal(denied, 1, 'lstat failures must not open the guide');
});

test('Read and write streams transfer a file in chunks for SFTP pipelines', async () => {
  const native = fakeNative();
  const fs = createLocalFs({invoke: native.invoke});
  const chunks = [];
  for await (const chunk of fs.createReadStream('/home/me/a.txt', {highWaterMark: 4})) chunks.push(Buffer.from(chunk));
  assert.equal(Buffer.concat(chunks).toString(), '0123456789');

  const writer = fs.createWriteStream('/home/me/copy.txt', {mode: 0o600});
  await new Promise((resolve, reject) => {
    writer.on('error', reject).on('finish', resolve);
    writer.write(Buffer.from('hello '));
    writer.write(Buffer.from('world'));
    writer.end();
  });
  assert.equal(native.files.get('/home/me/copy.txt').toString(), 'hello world');
  assert.equal(writer.bytesWritten, 11);

  const empty = fs.createWriteStream('/home/me/empty.txt');
  await new Promise(resolve => empty.on('finish', resolve).end());
  assert.equal(native.files.get('/home/me/empty.txt').length, 0, 'an empty transfer still creates the file');
});
