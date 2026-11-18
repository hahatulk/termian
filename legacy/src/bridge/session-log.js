// libtermius `terminalOutput`: encrypted terminal session logs. The recovered
// renderer writes every SSH/telnet/serial output chunk through `Writer`
// (ui-process `l9`) and replays a finished log with `LogReader` (`Poe`).
// Each log has its own random key, kept in the vault's history entry
// (`session_log_data.secretKey`); src/rust/session_logs.rs stores only the
// ciphertext in `session-logs-v2/`.
//
// File format: an 8-byte magic header, then records of
//   u32 big-endian length | 24-byte nonce | NaCl secretbox(chunk)
// A new header may follow if a writer reopens the file; a torn last record
// (crash while writing) is ignored on reading.
import nacl from 'tweetnacl';
import { Buffer } from 'buffer';

export const MAGIC = Buffer.from('TMNLOG1\n', 'ascii');
const NONCE = nacl.secretbox.nonceLength;
const KEY = nacl.secretbox.keyLength;
const FLUSH_BYTES = 64 * 1024;
const FLUSH_MS = 1000;
const REPLAY_CHUNK = 1024 * 1024;
const LOG_NAME = /^[A-Za-z0-9_-][A-Za-z0-9_.-]{0,127}$/;

export const MISSING_LOG = 'Файл лога сессии не найден: он удалён (хранится 60 дней) или не был записан.';

// The renderer passes `<sessionLogsPath>/<name>`; only the name reaches Rust.
export function logName(filename) {
  const name = String(filename ?? '').split(/[\\/]/).pop();
  if (!LOG_NAME.test(name)) throw new Error('Invalid session log name');
  return name;
}

function logKey(encryptionKey) {
  const key = encryptionKey == null ? null : new Uint8Array(Buffer.from(encryptionKey));
  return key?.length === KEY ? key : null;
}

export function encodeRecord(chunk, key) {
  const nonce = nacl.randomBytes(NONCE);
  const sealed = nacl.secretbox(new Uint8Array(chunk), nonce, key);
  const header = Buffer.alloc(4);
  header.writeUInt32BE(NONCE + sealed.length);
  return Buffer.concat([header, Buffer.from(nonce), Buffer.from(sealed)]);
}

export function decodeLog(bytes, key) {
  const input = Buffer.from(bytes);
  const chunks = [];
  let offset = 0;
  let error = null;
  while (offset < input.length) {
    if (input.subarray(offset, offset + MAGIC.length).equals(MAGIC)) {
      offset += MAGIC.length;
      continue;
    }
    if (offset + 4 > input.length) break;
    const length = input.readUInt32BE(offset);
    if (length <= NONCE || offset + 4 + length > input.length) break;
    const nonce = input.subarray(offset + 4, offset + 4 + NONCE);
    const sealed = input.subarray(offset + 4 + NONCE, offset + 4 + length);
    const plain = nacl.secretbox.open(new Uint8Array(sealed), new Uint8Array(nonce), key);
    if (!plain) {
      error = 'Не удалось расшифровать лог сессии';
      break;
    }
    chunks.push(Buffer.from(plain));
    offset += 4 + length;
  }
  return {data: Buffer.concat(chunks), error};
}

export function createTerminalOutput({ invoke, onLogFinished = () => {} }) {
  class Writer {
    constructor({ filename, encryptionKey, finishedCallback }) {
      this.finishedCallback = finishedCallback;
      this.pending = [];
      this.pendingBytes = 0;
      this.timer = null;
      this.ended = false;
      this.chain = Promise.resolve();
      try {
        this.name = logName(filename);
      } catch (error) {
        this.name = null;
      }
      this.key = logKey(encryptionKey);
      if (!this.name || !this.key) {
        console.error('[Termian] Session log disabled: invalid name or key');
        this.ended = true;
        return;
      }
      this._write(MAGIC);
    }

    _write(bytes) {
      const data = Buffer.from(bytes).toString('base64');
      this.chain = this.chain
        .then(() => invoke('session_log_append', {name: this.name, data}))
        .catch(error => console.error('[Termian] Session log write failed:', String(error?.message || error)));
      return this.chain;
    }

    append(data) {
      if (this.ended || data == null) return;
      const bytes = Buffer.from(data);
      if (!bytes.length) return;
      this.pending.push(bytes);
      this.pendingBytes += bytes.length;
      if (this.pendingBytes >= FLUSH_BYTES) void this.flush();
      else this.timer ??= setTimeout(() => void this.flush(), FLUSH_MS);
    }

    flush() {
      clearTimeout(this.timer);
      this.timer = null;
      if (!this.pending.length) return this.chain;
      const chunk = Buffer.concat(this.pending);
      this.pending = [];
      this.pendingBytes = 0;
      return this._write(encodeRecord(chunk, this.key));
    }

    end() {
      if (this.ended) {
        if (!this.name || !this.key) try { this.finishedCallback?.(); } catch {}
        return;
      }
      this.ended = true;
      void this.flush().then(() => {
        try { this.finishedCallback?.(); } catch (error) { console.error(error); }
        onLogFinished(this.name);
      });
    }
  }

  class LogReader {
    constructor({ filename, encryptionKey, onDataReady, onError }) {
      void this._replay(filename, encryptionKey, onDataReady, onError);
    }

    async _replay(filename, encryptionKey, onDataReady, onError) {
      // Every outcome ends with an empty chunk: the renderer leaves its
      // "fetching" state only after output arrives.
      const fail = message => {
        try { onError?.(null, null, message); } finally { onDataReady(Buffer.alloc(0)); }
      };
      let name;
      try { name = logName(filename); } catch { return fail(MISSING_LOG); }
      const key = logKey(encryptionKey);
      if (!key) return fail('Не удалось расшифровать лог сессии: ключ недоступен');
      const read = async () => Buffer.from(await invoke('session_log_read', {name}));
      let bytes;
      try {
        bytes = await read();
      } catch (error) {
        if (!/^ENOENT/.test(String(error?.message || error))) return fail(String(error?.message || error));
        const restored = await invoke('session_log_cloud_restore', {name}).catch(() => false);
        if (!restored) return fail(MISSING_LOG);
        try { bytes = await read(); } catch (retry) { return fail(String(retry?.message || retry)); }
      }
      const { data, error } = decodeLog(bytes, key);
      for (let offset = 0; offset < data.length; offset += REPLAY_CHUNK) {
        onDataReady(data.subarray(offset, offset + REPLAY_CHUNK));
      }
      if (error) return fail(error);
      onDataReady(Buffer.alloc(0));
    }
  }

  return {
    generateSecretStreamKey: () => Buffer.from(nacl.randomBytes(KEY)),
    Writer,
    LogReader,
  };
}
