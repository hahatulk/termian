import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { Buffer } from 'buffer';
import { createTerminalOutput, decodeLog, MAGIC, MISSING_LOG, logName } from '../src/bridge/session-log.js';
import { installSessionLogSync, sessionLogItems } from '../src/bridge/session-log-sync.js';
import { createFsPromises } from '../src/bridge/node-shims.js';

const base = path.resolve(import.meta.dirname, '..');
const source = relative => fs.readFileSync(path.resolve(base, relative), 'utf8');
const tick = (ms = 0) => new Promise(resolve => setTimeout(resolve, ms));

// In-memory stand-in for src/rust/session_logs.rs.
function fakeNative({ cloud = new Map() } = {}) {
  const files = new Map();
  const calls = [];
  const invoke = async (command, args) => {
    calls.push(command);
    switch (command) {
      case 'session_log_append':
        files.set(args.name, Buffer.concat([files.get(args.name) ?? Buffer.alloc(0), Buffer.from(args.data, 'base64')]));
        return null;
      case 'session_log_read': {
        const data = files.get(args.name);
        if (!data) throw `ENOENT: No such file or directory`;
        return data.buffer.slice(data.byteOffset, data.byteOffset + data.length);
      }
      case 'session_log_cloud_restore':
        if (!cloud.has(args.name)) return false;
        files.set(args.name, cloud.get(args.name));
        return true;
      default:
        throw new Error('unexpected command ' + command);
    }
  };
  return {files, calls, invoke};
}

function replay(output, filename, key) {
  return new Promise(resolve => {
    const chunks = [];
    const errors = [];
    new output.LogReader({
      filename, encryptionKey: key,
      onDataReady: data => {
        if (data.length) chunks.push(Buffer.from(data));
        else resolve({text: Buffer.concat(chunks).toString('utf8'), errors});
      },
      onError: (_a, _b, message) => errors.push(message),
    });
  });
}

test('Session log writer encrypts terminal output and the reader replays it', async () => {
  const native = fakeNative();
  const finished = [];
  const output = createTerminalOutput({invoke: native.invoke, onLogFinished: name => finished.push(name)});
  const key = output.generateSecretStreamKey();
  assert.equal(key.length, 32);
  let callback = 0;
  const writer = new output.Writer({
    filename: '/Users/local/Library/Application Support/app.termian.desktop/session-logs-v2/conn-1.log',
    encryptionKey: key, finishedCallback: () => callback++,
  });
  writer.append(Buffer.from('$ uname -a\r\n'));
  writer.append(Buffer.from('Linux h2 6.1 \u2014 ok\r\n'));
  writer.append(Buffer.alloc(70 * 1024, 'x'));  // forces an immediate flush
  writer.append(Buffer.from('tail'));
  writer.end();
  await tick(10);
  assert.equal(callback, 1);
  assert.deepEqual(finished, ['conn-1.log']);
  const file = native.files.get('conn-1.log');
  assert.ok(file.subarray(0, MAGIC.length).equals(MAGIC));
  assert.ok(!file.includes(Buffer.from('uname')), 'log contents are not stored in plain text');

  const { text, errors } = await replay(output, 'session-logs-v2/conn-1.log', key);
  assert.deepEqual(errors, []);
  assert.equal(text, '$ uname -a\r\nLinux h2 6.1 \u2014 ok\r\n' + 'x'.repeat(70 * 1024) + 'tail');

  // A reopened writer appends a second header; a torn last record is ignored.
  const again = new output.Writer({filename: 'conn-1.log', encryptionKey: key});
  again.append('more');
  again.end();
  await tick(10);
  native.files.set('conn-1.log', Buffer.concat([native.files.get('conn-1.log'), Buffer.from([0, 0, 0, 90, 1, 2])]));
  assert.ok((await replay(output, 'conn-1.log', key)).text.endsWith('tailmore'));

  const wrong = await replay(output, 'conn-1.log', Buffer.alloc(32, 1));
  assert.deepEqual(wrong.errors, ['Не удалось расшифровать лог сессии']);
});

test('A missing log never leaves the log tab fetching: cloud copy or a visible error', async () => {
  const output0 = createTerminalOutput({invoke: async () => null});
  const key = output0.generateSecretStreamKey();
  const donor = fakeNative();
  const { Writer } = createTerminalOutput({invoke: donor.invoke});
  const writer = new Writer({filename: 'restored.log', encryptionKey: key});
  writer.append('from another device');
  writer.end();
  await tick(10);

  const native = fakeNative({cloud: new Map([['restored.log', donor.files.get('restored.log')]])});
  const output = createTerminalOutput({invoke: native.invoke});
  assert.equal((await replay(output, 'restored.log', key)).text, 'from another device');
  assert.ok(native.calls.includes('session_log_cloud_restore'));

  const missing = await replay(output, 'gone.log', key);
  assert.equal(missing.text, '');
  assert.deepEqual(missing.errors, [MISSING_LOG]);
  // Logs recorded by builds without terminalOutput carry an unusable key.
  const broken = await replay(output, 'gone.log', Buffer.from('@termius/libtermius.terminalOutput'));
  assert.equal(broken.errors.length, 1);
});

test('Session log names cannot leave the log directory and bad keys disable writing', async () => {
  assert.equal(logName('C:\\data\\session-logs-v2\\a-1.log'), 'a-1.log');
  for (const bad of ['', '..', '.hidden', 'a b.log']) assert.throws(() => logName(bad));
  const native = fakeNative();
  const output = createTerminalOutput({invoke: native.invoke});
  let finished = 0;
  const writer = new output.Writer({filename: 'x.log', encryptionKey: Buffer.from('short'), finishedCallback: () => finished++});
  writer.append('secret');
  writer.end();
  await tick(5);
  assert.equal(native.files.size, 0);
  assert.equal(finished, 1, 'the renderer still learns the session finished');
  assert.deepEqual(decodeLog(Buffer.alloc(0), new Uint8Array(32)), {data: Buffer.alloc(0), error: null});
});

test('Background fs/promises reaches the real session log directory only', async () => {
  const calls = [];
  const fsp = createFsPromises({invoke: async (command, args) => {
    calls.push([command, args.op, args.name]);
    if (args.op === 'readdir') return ['a.log'];
    if (args.op === 'stat') return {size: 12, mtimeMs: 5000, isFile: true};
    if (args.op === 'access' && args.name === 'gone.log') throw 'ENOENT: not found';
    return null;
  }});
  const dir = 'termian-data/session-logs-v2';
  assert.deepEqual(await fsp.readdir(dir), ['a.log']);
  const stat = await fsp.stat(dir + '/a.log');
  assert.equal(stat.size, 12);
  assert.equal(stat.mtime.getTime(), 5000);
  await fsp.access(dir + '/a.log');
  await assert.rejects(fsp.access(dir + '/gone.log'), error => error.code === 'ENOENT');
  await fsp.unlink(dir + '/a.log');
  assert.deepEqual(calls.at(-1), ['session_logs_fs', 'unlink', 'a.log']);
  // Other paths keep the previous empty-directory behaviour.
  assert.deepEqual(await fsp.readdir('/Users/local/.ssh'), []);
  await assert.rejects(fsp.access('/Users/local/.ssh/id_rsa'), error => error.code === 'ENOENT');
  assert.equal(calls.length, 5);
});

test('Session logs are backed up silently, without snapshot status or credentials', async () => {
  const state = {entities: {histories: [
    {local_id: 1, label: 'h2.nexus DE', address: '10.0.0.5', port: 22, username: 'root', password: 'hunter2',
      ssh_key: {private_key: 'PRIVATE'}, protocol: 'ssh', connected_at: 1, disconnectedAt: 2,
      session_log_data: {name: 'a.log', secretKey: 'S2V5'}},
    {local_id: 2, label: 'no log'},
    {local_id: 3, command: {session_log_data: {name: 'b.log', secretKey: 'S2V5'}}},
  ]}};
  const items = sessionLogItems(state);
  assert.deepEqual(items.map(item => item.name), ['a.log', 'b.log']);
  assert.equal(items[0].meta.session_log_data.secretKey, 'S2V5');
  assert.ok(!JSON.stringify(items).includes('hunter2'));
  assert.ok(!JSON.stringify(items).includes('PRIVATE'));

  const calls = [];
  let reply = {configured: true, uploaded: 2};
  let autoSync = false;
  const timers = {setTimeout: () => 0, clearTimeout() {}, setInterval: () => 0, clearInterval() {}};
  const sync = installSessionLogSync({
    invoke: async (command, args) => {
      if (command === 'self_cloud_get_config') return {configured: true, autoSync};
      calls.push([command, args.items.length]); if (reply instanceof Error) throw reply; return reply;
    },
    getState: () => state, timers, isCredentialError: message => /токен доступа/.test(message),
  });
  assert.equal(await sync.run(), null, 'disabled auto-sync also stops log backups');
  assert.deepEqual(calls, []);
  autoSync = true;
  assert.deepEqual(await sync.run(), {configured: true, uploaded: 2});
  assert.deepEqual(calls, [['session_logs_cloud_sync', 2]]);

  reply = new Error('Отправка лога сессии: сервер отклонил токен доступа (HTTP 401)');
  const warn = console.warn;
  console.warn = () => {};
  try { assert.equal(await sync.run(), null); } finally { console.warn = warn; }
  assert.equal(await sync.run(), null, 'credential errors pause background uploads');
  assert.equal(calls.length, 2);
  sync.dispose();

  const old = installSessionLogSync({invoke: async () => ({configured: true, unsupported: true}), getState: () => state, timers});
  // (the fake also answers self_cloud_get_config: configured, auto-sync on)
  await old.run();
  assert.equal(await old.run(), null, 'servers without the log API are not asked again');
});

test('Recovered UI logs every session by default and the log tab cannot hang', () => {
  const saga = source('src/ui/assets/reconnectSaga-3b11f62e.js');
  assert.match(saga, /areSessionLogsEnabled = \(er15, tr\) => \{\s+var rr;\s+return [^;]+\.enabled\) !== false;/);
  const background = source('src/ui/assets/main-0debf4cf.js');
  const download = background.slice(background.indexOf('function* a1t('), background.indexOf('function* cEe('));
  assert.ok(!/upload_status\) !== "COMPLETED"\) return;/.test(download), 'download saga must always answer');
  assert.ok(download.includes('session_log_cloud_restore'));
  // Git for Windows may check the bundle out with CRLF line endings.
  assert.match(download, /qN\.failure\(true, n\.requestId\)\);\r?\n\s*return;/);
  const bridge = source('src/original-bridge.js');
  assert.ok(bridge.includes("createFsPromises({invoke})"));
  assert.ok(bridge.includes('terminalOutput:libtermius.terminalOutput'));
  assert.ok(source('src/bridge/ssh-compat.js').includes('terminalOutput: createTerminalOutput('));
  // Logs stay out of the vault snapshot: uploads cannot raise header badges.
  const logSync = source('src/bridge/session-log-sync.js');
  for (const snapshotCall of ['self_cloud_upload', 'self_cloud_download', 'self_cloud_fetch', 'saveBaseline', 'saveMergeState']) {
    assert.ok(!logSync.includes(snapshotCall), snapshotCall);
  }
  const rust = source('src/rust/lib.rs');
  for (const command of ['session_log_append', 'session_log_read', 'session_logs_fs', 'session_logs_cloud_sync', 'session_log_cloud_restore']) {
    assert.ok(rust.includes('session_logs::' + command), command);
  }
  assert.ok(rust.includes('session_logs::start_retention(app.handle())'));
  assert.match(source('src/rust/session_logs.rs'), /RETENTION: Duration = Duration::from_secs\(60 \* 24 \* 60 \* 60\)/);
});
