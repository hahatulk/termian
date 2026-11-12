// Replacement for the proprietary native @termius/libtermius module.
// SshClient keeps libtermius' callback contract, which the recovered renderer
// (ui-process `are`/`$re` classes) drives; the protocol itself runs in Rust
// (src/rust/ssh_client.rs) and talks back over a Tauri channel.
import { Buffer } from 'buffer';
import { Channel } from '@tauri-apps/api/core';
import { createKeyHelpers } from './ssh-compat-keys.js';
import { createTerminalOutput } from './session-log.js';

// Numeric values only need to be distinct: the renderer compares them with
// the same object (`r9` in ui-process) to pick a log icon.
export const logging = Object.freeze({
  'Severity::Info': 0,
  'Severity::Warning': 1,
  'Severity::Error': 2,
  'Visibility::User': 0,
  'Visibility::Debug': 1,
});

const KNOWN_HOST_STATUS = {mismatch: 1, notfound: 2, failure: 3};

// Advertised for the "exclude unstable PQC KEX" option; russh negotiates its
// own list, so this is informational only.
const CLIENT_KEX = [
  'mlkem768x25519-sha256', 'curve25519-sha256', 'curve25519-sha256@libssh.org',
  'ecdh-sha2-nistp256', 'ecdh-sha2-nistp384', 'ecdh-sha2-nistp521',
  'diffie-hellman-group-exchange-sha256', 'diffie-hellman-group16-sha512',
  'diffie-hellman-group14-sha256',
];

// Opt-in protocol tracing: localStorage.setItem('termian:debug-ssh', '1').
const debugEvents = (() => { try { return localStorage.getItem('termian:debug-ssh') === '1'; } catch { return false; } })();

const toBase64 =data => Buffer.from(typeof data === 'string' ? Buffer.from(data, 'utf8') : data).toString('base64');

function errorObject(message, type = 'connection') {
  const error = new Error(String(message || 'SSH connection failed'));
  error.libtermius_error_type = type;
  error.libtermius_error_code = 0;
  return error;
}

export function createLibtermius({ crypto, invoke, onSessionLogFinished }) {
  class SshClient {
    static getClientSshMethods(type) {
      return type === 'kex' ? [...CLIENT_KEX] : [];
    }

    constructor() {
      this.id = globalThis.crypto.randomUUID();
      this.options = null;
      this.connected = false;
      this.closed = false;
      this.nextChannel = 1;
      this.channels = new Map();
      this.sessionInfo = null;
    }

    getServerSshMethods() { return []; }
    getAgreedSshMethod() { return ''; }

    _log(message, severity = logging['Severity::Info'], visibility = logging['Visibility::User']) {
      if (this.options?.enableLogging !== false) {
        try { this.options?.loggingCallback?.(visibility, severity, message); } catch (error) { console.error(error); }
      }
    }

    _reply(request, value) {
      if (this.closed) return;
      void invoke('ssh_client_reply', {id: this.id, request, value: value ?? null})
        .catch(error => console.error('[Termian] SSH reply failed:', error));
    }

    _onEvent(event) {
      if (this.closed && event.kind !== 'disconnected') return;
      const options = this.options;
      switch (event.kind) {
        case 'log':
          this._log(event.message,
            event.severity === 'error' ? logging['Severity::Error'] : event.severity === 'warning' ? logging['Severity::Warning'] : logging['Severity::Info'],
            event.visibility === 'debug' ? logging['Visibility::Debug'] : logging['Visibility::User']);
          break;
        case 'sessionInfo':
          this.sessionInfo = {softwareVersion: event.softwareVersion, protocolVersion: event.protocolVersion};
          options.sessionInfoCallback?.(this.sessionInfo);
          break;
        case 'hostKey': {
          let answered = false;
          const answer = accepted => {
            if (answered) return;
            answered = true;
            this._reply(event.request, !!accepted);
          };
          if (typeof options.knownhostsCallback !== 'function') { answer(false); break; }
          options.knownhostsCallback(KNOWN_HOST_STATUS[event.status] ?? KNOWN_HOST_STATUS.failure,
            event.fingerprint, event.publicKey, event.keyType, undefined, answer);
          break;
        }
        case 'authMethods': {
          if (typeof options.authInteractiveCallback !== 'function') { this._reply(event.request, null); break; }
          options.authInteractiveCallback(event.methods, choice => this._reply(event.request, choice ?? null));
          break;
        }
        case 'keyboardInteractive': {
          if (typeof options.keyboardInteractiveCallback !== 'function') { this._reply(event.request, null); break; }
          options.keyboardInteractiveCallback(event.name, event.instruction, event.prompts,
            answers => this._reply(event.request, Array.isArray(answers)
              ? answers.map(answer => ({text: answer?.text == null ? '' : String(answer.text)}))
              : null));
          break;
        }
        case 'authKeyState':
          options.authKeyStateCallback?.(event.index, event.state);
          options.authInfoCallback?.('publickey', event.state);
          break;
        case 'connected':
          this.connected = true;
          options.connectCallback?.();
          break;
        case 'error':
          // Keep the exact transport/handshake error visible in the original
          // connection-flow log, not only in the final error callback.
          this._log(event.message, logging['Severity::Error']);
          this._finish(() => options.errorCallback?.(errorObject(event.message)));
          break;
        case 'disconnected':
          this._finish(() => options.disconnectCallback?.());
          break;
        default:
          this._onChannelEvent(event);
      }
    }

    _onChannelEvent(event) {
      const channel = this.channels.get(event.channel ?? event.forward);
      if (!channel) return;
      switch (event.kind) {
        case 'forwardReady':
          channel.forwardReady?.(event);
          break;
        case 'forwardStopped':
          this.channels.delete(event.forward);
          channel.closed?.();
          break;
        case 'channelReady':
          channel.ready?.();
          break;
        case 'data':
          channel.data?.(Buffer.from(event.data, 'base64'));
          break;
        case 'execResult':
          this.channels.delete(event.channel);
          channel.completion?.(event.exitCode, Buffer.from(event.data, 'base64'));
          break;
        case 'channelClosed':
          this.channels.delete(event.channel);
          channel.closed?.(event.exitStatus);
          break;
        case 'channelError':
          this.channels.delete(event.channel);
          channel.error?.(errorObject(event.message, 'channel'));
          break;
      }
    }

    // Every terminal path (error, server disconnect, close) ends the open
    // channels exactly once so the renderer's shells/finalizers run.
    _finish(notify) {
      if (this.finished) return;
      this.finished = true;
      for (const [id, channel] of this.channels) {
        this.channels.delete(id);
        try { channel.closed?.(null); } catch (error) { console.error(error); }
      }
      try { notify(); } catch (error) { console.error(error); }
    }

    connect(options) {
      this.options = options;
      // An explicit Verbose (-v) retry arms this for exactly one connection;
      // regular retries never inherit verbose logging.
      const verbose = globalThis.__termianSshVerboseNext === true;
      globalThis.__termianSshVerboseNext = false;
      const events = new Channel();
      events.onmessage = event => {
        if (debugEvents) console.debug('[Termian SSH]', this.id, event.kind, event.channel ?? '', event.request ?? '');
        try { this._onEvent(event); }
        catch (error) { console.error('[Termian] SSH event handler failed:', event?.kind, error); }
      };
      if (options.proxy) {
        queueMicrotask(() => this._finish(() => options.errorCallback?.(errorObject('Proxy connections are not supported in this build yet'))));
        return;
      }
      const port = Number.parseInt(options.port, 10) || 22;
      const alias = options.knownhostAlias;
      const environment = Object.entries(options.environmentVariables ?? {})
        .filter(([name]) => typeof name === 'string' && name);
      void invoke('ssh_client_connect', {
        id: this.id,
        events,
        options: {
          host: String(options.host ?? '').trim(),
          port,
          verbose,
          username: String(options.username ?? ''),
          password: options.password || null,
          keys: (options.keys ?? []).map(key => ({privateKey: key.privateKey || '', passphrase: key.passphrase || null})),
          knownHosts: (options.knownhosts ?? []).filter(line => typeof line === 'string'),
          knownHostAlias: alias?.host ? {host: String(alias.host).trim(), port: Number.parseInt(alias.port, 10) || port} : null,
          keepaliveInterval: Math.max(0, Number(options.keepaliveInterval) || 0),
          environment: environment.map(([name, value]) => [String(name), String(value ?? '')]),
        },
      }).catch(error => {
        const message = error?.message || String(error);
        this._log(message, logging['Severity::Error']);
        this._finish(() => options.errorCallback?.(errorObject(message)));
      });
    }

    close() {
      if (this.closed) return;
      this.closed = true;
      void invoke('ssh_client_close', {id: this.id}).catch(() => {});
      const options = this.options;
      if (options) this._finish(() => options.disconnectCallback?.());
    }

    _openChannel(handlers) {
      const id = this.nextChannel++;
      this.channels.set(id, handlers);
      return id;
    }

    _channelApi(id) {
      return {
        sendData: data => {
          if (data == null || this.closed) return;
          void invoke('ssh_client_write', {id: this.id, channel: id, data: toBase64(data)}).catch(() => {});
        },
        sendSize: (cols, rows) => {
          if (this.closed || !(cols > 0 && rows > 0)) return;
          void invoke('ssh_client_resize', {id: this.id, channel: id, cols: Math.floor(cols), rows: Math.floor(rows)}).catch(() => {});
        },
        close: () => void invoke('ssh_client_close_channel', {id: this.id, channel: id}).catch(() => {}),
      };
    }

    shell(options) {
      const pty = options.ptyOptions ?? {};
      const id = this._openChannel({
        ready: () => options.readyCallback?.(this._channelApi(id)),
        data: buffer => options.dataCallback?.(buffer),
        closed: status => {
          try { options.disconnectCallback?.(status); } finally { options.finalizeCallback?.(); }
        },
        error: error => {
          options.errorCallback?.(error);
          options.finalizeCallback?.();
        },
      });
      const environment = Object.entries(this.options?.environmentVariables ?? {})
        .map(([name, value]) => [String(name), String(value ?? '')]);
      void invoke('ssh_client_shell', {
        id: this.id, channel: id, environment,
        pty: {term: String(pty.term || 'xterm-256color'), cols: Number(pty.columns) || 80, rows: Number(pty.rows) || 24},
      }).catch(error => this._onChannelEvent({kind: 'channelError', channel: id, message: error?.message || String(error)}));
    }

    exec(options) {
      const id = this._openChannel({
        completion: (exitCode, data) => options.completionCallback?.(exitCode, data),
        closed: () => options.errorCallback?.(errorObject('Exec channel closed', 'channel')),
        error: error => options.errorCallback?.(error),
      });
      void invoke('ssh_client_exec', {id: this.id, channel: id, command: toBase64(options.command ?? '')})
        .catch(error => this._onChannelEvent({kind: 'channelError', channel: id, message: error?.message || String(error)}));
    }

    interactiveExec(options) { this.exec(options); }

    sftp(options) {
      const sftp = globalThis.crypto.randomUUID();
      void invoke('sftp_start', {id: this.id, sftp})
        .then(() => {
          const fs = createSftpFileSystem(invoke, sftp, () => options.disconnectCallback?.());
          this.channels.set('sftp:' + sftp, {closed: () => fs.close()});
          options.readyCallback?.(fs);
        })
        .catch(error => options.errorCallback?.(errorObject(error?.message || error, 'channel')));
    }

    _forward(kind, options) {
      const forward = this.nextChannel++;
      this.channels.set(forward, {
        forwardReady: ({bindAddress, bindPort}) => options.readyCallback?.({
          getBindAddress: () => bindAddress,
          getBindPort: () => bindPort,
        }),
        closed: () => options.disconnectCallback?.(),
      });
      void invoke('ssh_forward_start', {
        id: this.id, forward,
        options: {
          kind,
          bindAddress: String(options.bindAddress ?? ''),
          port: Number(options.port) || 0,
          host: String(options.host ?? ''),
          hostPort: Number(options.hostPort) || 0,
        },
      }).catch(error => {
        this.channels.delete(forward);
        options.errorCallback?.(errorObject(error?.message || error, 'channel'));
      });
    }
    localPortForwarding(options) { this._forward('local', options); }
    remotePortForwarding(options) { this._forward('remote', options); }
    dynamicPortForwarding(options) { this._forward('dynamic', options); }
  }

  class SshAgentStorage {
    addKey() {}
    removeKey() {}
    clear() {}
  }

  return {
    ...createKeyHelpers(invoke),
    crypto,
    logging,
    sep: inertSep(),
    terminalOutput: createTerminalOutput({invoke, onLogFinished: onSessionLogFinished}),
    SshClient,
    SshAgentStorage,
  };
}

// libtermius' SFTP object: callback style with SFTP status codes (0 = ok),
// used by the renderer's `Ore` adapter and its read/write streams.
const S_IFMT = 0o170000;
const S_IFDIR = 0o040000;
const S_IFLNK = 0o120000;

function sftpAttributes(attrs = {}) {
  return {
    ...attrs,
    isDirectory: () => (attrs.mode & S_IFMT) === S_IFDIR,
    isSymbolicLink: () => (attrs.mode & S_IFMT) === S_IFLNK,
    isFile: () => (attrs.mode & S_IFMT) === 0o100000,
  };
}

export function createSftpFileSystem(invoke, sftp, onClosed) {
  let closed = false;
  const op = (name, args = {}) => closed
    ? Promise.resolve({code: 6, message: 'SFTP session is closed'})
    : invoke('sftp_op', {sftp, op: name, args}).catch(error => ({code: 7, message: String(error?.message || error)}));
  const status = (name, args, callback) => void op(name, args).then(result => callback?.(result.code));

  function getFile() {
    if (closed) return null;
    let handle = null;
    let offset = 0;
    // Streams call read/write right after open(); hold them until the handle
    // exists, otherwise they race the open round-trip and fail at random.
    let opening = Promise.resolve(0);
    const whenOpen = run => void opening.then(code => (code ? run(null, code) : run(handle, 0)));
    return {
      // Resume support: the next read/write starts at this byte offset.
      setOffset(position) { offset = Number(position) || 0; },
      open(path, flags, mode, callback) {
        opening = op('open', {path, flags: Number(flags) || 0, mode: Number(mode) || 0}).then(result => {
          handle = result.handle ?? null;
          callback?.(result.code);
          return result.code;
        });
      },
      read(length, callback) {
        whenOpen((current, failed) => {
          if (failed) return void callback?.(Buffer.alloc(0), false, failed);
          void op('read', {handle: current, offset, length: Number(length) || 65536}).then(result => {
            const data = Buffer.from(result.data ?? '', 'base64');
            offset += data.length;
            callback?.(data, !!result.eof, result.code);
          });
        });
      },
      write(data, isLast, callback) {
        if (data == null || data.length === 0) {
          whenOpen((current, failed) => callback?.(failed));
          return;
        }
        const bytes = Buffer.from(data);
        const position = offset;
        offset += bytes.length;
        whenOpen((current, failed) => {
          if (failed) return void callback?.(failed);
          void op('write', {handle: current, offset: position, data: bytes.toString('base64')}).then(result => callback?.(result.code));
        });
      },
      close(callback) {
        void opening.then(() => {
          if (!handle) return void callback?.(0);
          const current = handle;
          handle = null;
          void op('close', {handle: current}).then(result => callback?.(result.code));
        });
      },
    };
  }

  return {
    listDir(path, callback) {
      void op('listDir', {path}).then(result => callback?.(result.code,
        (result.entries ?? []).map(entry => ({...entry, attrs: sftpAttributes(entry.attrs)}))));
    },
    realPath(path, callback) { void op('realPath', {path}).then(result => callback?.(result.code, result.path)); },
    readLink(path, callback) { void op('readLink', {path}).then(result => callback?.(result.code, result.path)); },
    stat(path, callback) { void op('stat', {path}).then(result => callback?.(result.code, sftpAttributes(result.attrs))); },
    lstat(path, callback) { void op('lstat', {path}).then(result => callback?.(result.code, sftpAttributes(result.attrs))); },
    symlink(target, path, callback) { status('symlink', {path, target}, callback); },
    removeDir(path, callback) { status('removeDir', {path}, callback); },
    removeFile(path, callback) { status('removeFile', {path}, callback); },
    rename(path, target, callback) { status('rename', {path, target}, callback); },
    makeDir(path, mode, callback) { status('makeDir', {path, mode: Number(mode) || 0o755}, callback); },
    chmod(path, mode, callback) { status('chmod', {path, mode: Number(mode) || 0}, callback); },
    getFile,
    close() {
      if (closed) return;
      closed = true;
      void invoke('sftp_stop', {sftp}).catch(() => {});
      onClosed?.();
    },
  };
}

// Secure Enclave helpers exist only on macOS hardware builds of libtermius.
function inertSep() {
  return {
    isSupported: () => false,
    generateNewKey: async () => { throw new Error('Secure Enclave keys are not supported'); },
    fetchAllKeys: async () => [],
  };
}
