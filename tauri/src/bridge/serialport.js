// Replacement for the `@serialport/stream` SerialPort class used by the
// renderer's serial provider (ui-process `Roe`) and the port picker (`list()`).
// The port itself is opened by the Rust serial worker (src/rust/lib.rs).
import { Buffer } from 'buffer';
import { EventEmitter } from 'events';

export function createSerialPort({ invoke, listen }) {
  let events = null;
  const ports = new Map();

  function subscribe() {
    if (!events) {
      events = listen('termian-terminal', ({payload}) => {
        const port = ports.get(payload?.sessionId);
        if (port) port._onNative(payload.kind, payload.data);
      });
    }
    return events;
  }

  function nativeOptions(path, options) {
    return {
      port: path,
      baudRate: Number(options.baudRate) || 9600,
      dataBits: Number(options.dataBits) || 8,
      stopBits: Number(options.stopBits) || 1,
      parity: String(options.parity || 'none'),
      flowControl: options.rtscts ? 'rts/cts' : options.xon || options.xoff ? 'xon/xoff' : 'none',
    };
  }

  class SerialPort extends EventEmitter {
    static async list() {
      return (await invoke('list_serial_ports')).map(path => ({path}));
    }

    constructor(path, options = {}) {
      super();
      this.path = path;
      this.options = options;
      this.sessionId = globalThis.crypto.randomUUID();
      this.isOpen = false;
    }

    open(callback) {
      ports.set(this.sessionId, this);
      this._openCallback = callback;
      void subscribe()
        .then(() => invoke('start_terminal', {
          sessionId: this.sessionId,
          connection: {serial: nativeOptions(this.path, this.options), shell: null},
          size: {cols: 80, rows: 24},
        }))
        .catch(error => this._fail(error));
    }

    _fail(error) {
      const failure = error instanceof Error ? error : new Error(String(error));
      ports.delete(this.sessionId);
      const callback = this._openCallback;
      this._openCallback = null;
      if (callback) callback(failure);
      else this.emit('error', failure);
    }

    _onNative(kind, data) {
      if (kind === 'status' && data === 'connected') {
        this.isOpen = true;
        const callback = this._openCallback;
        this._openCallback = null;
        callback?.(null);
        this.emit('open');
      } else if (kind === 'output') {
        this.emit('data', Buffer.from(data, 'base64'));
      } else if (kind === 'error') {
        this._fail(new Error(data));
      } else if (kind === 'status') {
        this._closed();
      }
    }

    _closed() {
      if (!ports.delete(this.sessionId)) return;
      this.isOpen = false;
      this.emit('close');
    }

    write(data, callback) {
      const bytes = Buffer.from(typeof data === 'string' ? Buffer.from(data, 'utf8') : data);
      void invoke('write_terminal', {sessionId: this.sessionId, data: bytes.toString('base64'), encoding: 'base64'})
        .then(() => callback?.(null), error => callback?.(new Error(String(error))));
      return true;
    }

    // DTR/DSR line control is not exposed by the native worker.
    set(_options, callback) { callback?.(null); }

    close(callback) {
      void invoke('stop_terminal', {sessionId: this.sessionId}).catch(() => {});
      this._closed();
      callback?.(null);
    }
  }

  return SerialPort;
}
