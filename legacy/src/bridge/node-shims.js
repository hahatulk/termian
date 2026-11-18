// Minimal Node.js module replacements for the recovered background process.

function enoent(path) {
  const error = new Error(`ENOENT: no such file or directory, '${path}'`);
  error.code = 'ENOENT';
  return error;
}

// The background process uses fs/promises for the session-log directory
// (file statuses, folder size, deleting logs) and for reading private keys
// referenced from ~/.ssh/config. Session logs live in src/rust/session_logs.rs;
// anything else behaves like an empty, writable directory.
const SESSION_LOG_PATH = /(?:^|[\\/])session-logs-v2(?:[\\/]([^\\/]+))?[\\/]?$/;

function nativeError(error, path) {
  const message = String(error?.message || error || 'EIO');
  const failure = new Error(message.includes(path) ? message : `${message}, '${path}'`);
  failure.code = /^E[A-Z]+/.exec(message)?.[0] ?? 'EIO';
  return failure;
}

export function createFsPromises({ invoke } = {}) {
  const sessionLog = path => invoke ? SESSION_LOG_PATH.exec(String(path ?? '')) : null;
  const logOp = (op, match, path) => invoke('session_logs_fs', {op, name: match[1] ?? null})
    .catch(error => { throw nativeError(error, path); });
  return {
    mkdir: async () => undefined,
    readdir: async path => {
      const match = sessionLog(path);
      return match && !match[1] ? logOp('readdir', match, path) : [];
    },
    access: async path => {
      const match = sessionLog(path);
      if (!match) throw enoent(path);
      await logOp('access', match, path);
    },
    stat: async path => {
      const match = sessionLog(path);
      if (!match) throw enoent(path);
      const stat = await logOp('stat', match, path);
      return {
        size: stat.size, mtimeMs: stat.mtimeMs, mtime: new Date(stat.mtimeMs),
        isFile: () => stat.isFile, isDirectory: () => !match[1],
      };
    },
    readFile: async path => { throw enoent(path); },
    writeFile: async () => undefined,
    unlink: async path => {
      const match = sessionLog(path);
      if (match?.[1]) await logOp('unlink', match, path);
    },
    rm: async () => undefined,
  };
}

// @termius/keytar backed by the OS credential store through Rust.
export function createKeytar(invoke) {
  return {
    getPassword: (service, account) => invoke('keychain_get', {service, account}),
    setPassword: (service, account, password) => invoke('keychain_set', {service, account, password}),
    deletePassword: (service, account) => invoke('keychain_delete', {service, account}),
    findCredentials: service => invoke('keychain_find', {service}),
  };
}

// fs.constants: the SFTP streams combine these into open flags, which
// src/rust/sftp.rs translates back (keep both sides in sync).
export const fsConstants = Object.freeze({
  F_OK: 0, R_OK: 4, W_OK: 2, X_OK: 1,
  O_RDONLY: 0, O_WRONLY: 1, O_RDWR: 2,
  O_CREAT: 64, O_EXCL: 128, O_TRUNC: 512, O_APPEND: 1024,
});
