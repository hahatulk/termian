// Node `fs` for the SFTP screen's "Local" panel (ui-process `uC`/`Kd`), backed
// by src/rust/local_fs.rs. The UI promisifies the callback functions and
// pipes createReadStream/createWriteStream between local and SFTP sessions.
// Synchronous calls (existsSync, …) stay unsupported: Tauri IPC is async.
import { Buffer } from 'buffer';
import { Readable, Writable } from 'readable-stream';

const CHUNK = 256 * 1024;
const MAX_CHUNK = 1024 * 1024;

// Rust answers {code, message}; the UI checks error.code like Node's fs.
export function toFsError(raw, path) {
  if (raw instanceof Error) return raw;
  const error = new Error(String(raw?.message || raw || 'File system error'));
  error.code = raw?.code || 'EIO';
  if (path !== undefined) error.path = path;
  return error;
}

function stats(raw) {
  const atime = new Date(raw.atimeMs), mtime = new Date(raw.mtimeMs);
  return {
    mode: raw.mode, uid: raw.uid, gid: raw.gid, size: raw.size,
    atimeMs: raw.atimeMs, mtimeMs: raw.mtimeMs, ctimeMs: raw.mtimeMs, birthtimeMs: raw.mtimeMs,
    atime, mtime, ctime: mtime, birthtime: mtime,
    isDirectory: () => raw.isDirectory,
    isFile: () => raw.isFile,
    isSymbolicLink: () => raw.isSymbolicLink,
    isBlockDevice: () => false, isCharacterDevice: () => false, isFIFO: () => false, isSocket: () => false,
  };
}

// Denials the user actually hits (entering a folder, copying a file). The UI
// lstat()s every listed entry and ignores failures, so those never prompt.
const VISIBLE_OPERATIONS = new Set(['realpath', 'readdir', 'read', 'truncate', 'write', 'mkdir', 'rename', 'unlink', 'rmdir']);

export function createLocalFs({ invoke, onPermissionDenied = () => {} }) {
  const denied = (error, op) => {
    if ((error.code === 'EPERM' || error.code === 'EACCES') && VISIBLE_OPERATIONS.has(op)) onPermissionDenied(error);
    return error;
  };
  const call = (op, path, extra = {}) => invoke('local_fs', { op, path: String(path ?? ''), ...extra })
    .catch(raw => { throw denied(toFsError(raw, path), op); });
  // Node style: the callback is always the last argument; options are optional.
  const callback = (args) => {
    const done = args[args.length - 1];
    if (typeof done !== 'function') throw new TypeError('Callback must be a function');
    return done;
  };
  const wrap = (work) => (...args) => {
    const done = callback(args);
    work(...args.slice(0, -1)).then(value => done(null, value), error => done(error));
  };

  async function mkdir(path, options) {
    const mode = typeof options === 'object' ? options?.mode : options;
    if (!(typeof options === 'object' && options?.recursive)) return void await call('mkdir', path, { mode });
    const parts = String(path).split('/');
    for (let index = 1; index <= parts.length; index++) {
      const prefix = parts.slice(0, index).join('/');
      if (!prefix || /^[A-Za-z]:$/.test(prefix)) continue;
      try { await call('mkdir', prefix); }
      catch (error) { if (error.code !== 'EEXIST') throw error; }
    }
  }

  function createReadStream(path, options = {}) {
    const size = Math.min(Math.max(options.highWaterMark || CHUNK, 16 * 1024), MAX_CHUNK);
    let position = options.start ?? 0;
    const end = options.end ?? Infinity; // inclusive, like Node
    let reading = false;
    const stream = new Readable({
      highWaterMark: size,
      read() {
        if (reading) return;
        const length = Math.min(size, end - position + 1);
        if (length <= 0) return void this.push(null);
        reading = true;
        invoke('local_fs_read', { path, offset: position, length }).then(buffer => {
          reading = false;
          const bytes = Buffer.from(new Uint8Array(buffer));
          if (!bytes.length) return void this.push(null);
          position += bytes.length;
          stream.bytesRead = position;
          this.push(bytes);
        }, raw => this.destroy(denied(toFsError(raw, path), 'read')));
      },
    });
    stream.path = path;
    stream.bytesRead = 0;
    queueMicrotask(() => { stream.emit('open'); stream.emit('ready'); });
    return stream;
  }

  function createWriteStream(path, options = {}) {
    let position = 0;
    // Create/truncate up front, like Node; 'open' fires once the file exists.
    const opened = call('truncate', path, { mode: options.mode });
    const stream = new Writable({
      highWaterMark: MAX_CHUNK,
      write(chunk, _encoding, done) {
        const bytes = Buffer.from(chunk);
        opened
          .then(() => invoke('local_fs_write', { path, offset: position, data: bytes.toString('base64') }))
          .then(() => { position += bytes.length; stream.bytesWritten = position; done(); },
            raw => done(denied(toFsError(raw, path), 'write')));
      },
      final(done) { opened.then(() => done(), error => done(error)); },
    });
    stream.path = path;
    stream.bytesWritten = 0;
    opened.then(() => { stream.emit('open'); stream.emit('ready'); }, error => stream.destroy(error));
    return stream;
  }

  return {
    realpath: wrap(path => call('realpath', path)),
    readdir: wrap(path => call('readdir', path)),
    lstat: wrap(async path => stats(await call('lstat', path))),
    stat: wrap(async path => stats(await call('stat', path))),
    access: wrap(path => call('access', path).then(() => undefined)),
    mkdir: wrap(mkdir),
    rmdir: wrap(path => call('rmdir', path).then(() => undefined)),
    unlink: wrap(path => call('unlink', path).then(() => undefined)),
    rename: wrap((from, to) => call('rename', from, { target: String(to) }).then(() => undefined)),
    chmod: wrap((path, mode) => call('chmod', path, { mode: typeof mode === 'string' ? parseInt(mode, 8) : mode }).then(() => undefined)),
    readlink: wrap(path => call('readlink', path)),
    // fs.symlink(target, path): `path` becomes a link pointing at `target`.
    symlink: wrap((target, path) => call('symlink', path, { target: String(target) }).then(() => undefined)),
    createReadStream,
    createWriteStream,
  };
}
