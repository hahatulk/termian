import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createSftpFileSystem } from '../src/bridge/ssh-compat.js';

const base = path.resolve(import.meta.dirname, '..');
const bundle = () => fs.readFileSync(path.join(base, 'src/ui/assets/ui-process-62a480c2.js'), 'utf8');

// Fake `sftp_op` invoke whose `open` answers only when the test releases it.
function fakeInvoke() {
  const calls = [];
  let release;
  const opened = new Promise(resolve => { release = resolve; });
  const invoke = async (command, {op, args} = {}) => {
    if (command !== 'sftp_op') return {};
    calls.push({op, args});
    if (op === 'open') { await opened; return {code: 0, handle: 'h1'}; }
    if (op === 'stat') return {code: 0, attrs: {size: 5, mode: 0o100644}};
    return {code: 0};
  };
  return {calls, invoke, release};
}

const callbackResult = fn => new Promise(resolve => fn(resolve));

test('SFTP file writes wait for the open handle instead of racing it', async () => {
  const {calls, invoke, release} = fakeInvoke();
  const file = createSftpFileSystem(invoke, 's1').getFile();
  file.open('/remote/a.txt', 0o1101, 0o644, () => {});
  const written = callbackResult(done => file.write(Buffer.from('hello'), false, done));
  await new Promise(resolve => setTimeout(resolve, 10));
  assert.equal(calls.some(call => call.op === 'write'), false, 'write must not be sent before open returns a handle');
  release();
  assert.equal(await written, 0);
  const write = calls.find(call => call.op === 'write');
  assert.equal(write.args.handle, 'h1');
  assert.equal(write.args.offset, 0);
});

test('SFTP file resumes reads and writes from a byte offset', async () => {
  const {calls, invoke, release} = fakeInvoke();
  release();
  const file = createSftpFileSystem(invoke, 's1').getFile();
  file.setOffset(4096);
  assert.equal(await callbackResult(done => file.open('/remote/a.txt', 0o101, 0o644, done)), 0);
  await callbackResult(done => file.write(Buffer.from('tail'), false, done));
  await callbackResult(done => file.read(10, () => done()));
  assert.equal(calls.find(call => call.op === 'write').args.offset, 4096);
  assert.equal(calls.find(call => call.op === 'read').args.offset, 4100);
});

test('SFTP file reports the failing open to queued writes', async () => {
  const invoke = async (command, {op}) => op === 'open' ? {code: 3} : {code: 0};
  const file = createSftpFileSystem(invoke, 's1').getFile();
  file.open('/denied', 0o1101, 0o644, () => {});
  assert.equal(await callbackResult(done => file.write(Buffer.from('x'), false, done)), 3);
});

test('Transfer UI resumes partial files, retries automatically and can always be discarded', () => {
  const ui = bundle();
  // Remote and local streams accept a start offset; resumed writes do not truncate.
  assert.match(ui, /createWriteStream", \(e10, \{ mode: n, start: o \} = \{\}\) => new Mre\(this\.fs, e10, n, o\)/);
  assert.match(ui, /createReadStream", \(e10, n = \{\}\) => new kre\(this\.fs, e10, n\.start\)/);
  assert.ok(ui.includes('const Rre = J1.O_WRONLY | J1.O_CREAT | J1.O_TRUNC, RreResume = J1.O_WRONLY | J1.O_CREAT;'));
  // Transfer stream: resume offset, bounded automatic retries, non-blocking discard.
  assert.ok(ui.includes('async resumeOffset('));
  assert.ok(ui.includes('TRANSFER_AUTO_RETRIES'));
  assert.ok(ui.includes('this.isClosed = true'));
  assert.doesNotMatch(ui, /await this\.destination\.client\.unlink\(n\);/, 'discard must not wait on a dead connection');
  // A dedicated transfers modal lists every stream.
  assert.ok(ui.includes('title: "Transfers"'));
  assert.ok(ui.includes('"Show all transfers"'));
});
