// Bind the ORIGINAL Termius 9.35.4 xterm/Redux UI of *local* terminal tabs to
// the native Tauri PTY backend. Electron ran them in a node-pty utility
// process; SSH tabs use the original connection flow through
// bridge/ssh-compat.js instead. No recovered HTML/CSS/JS asset is modified.
import { Buffer } from 'buffer';

const WAKE_ACTIONS = new Set([
  'OPEN_LOCAL_TERMINAL', 'OPEN_CONNECTION', 'ADD_TERMINAL_TAB',
  'ACTIVATE_TAB', 'OPEN_TAB', 'UPDATE_TAB', 'PATCH_TERMINAL',
  'RECONNECT_TAB', 'RECONNECT_TERMINAL',
]);

export function findOriginalTerminalApi(id, doc = document) {
  for (const element of doc.querySelectorAll('.xterm')) {
    let ancestor = element.parentElement;
    for (let depth = 0; depth < 6 && ancestor; depth++, ancestor = ancestor.parentElement) {
      const key = Object.keys(ancestor).find(name => name.startsWith('__reactFiber$'));
      if (!key) continue;
      let fiber = ancestor[key];
      for (let step = 0; fiber && step < 10; step++, fiber = fiber.return) {
        const api = fiber.memoizedProps?.terminalApi;
        if (api?.id === id && api.terminal?.write && api.terminal?.onData) return api;
      }
    }
  }
  return null;
}

// Session-log settings of a local tab, from the original terminal component
// (its `connection` prop sits above the xterm element of the same terminal).
export function findOriginalSessionLogData(id, doc = document) {
  for (const element of doc.querySelectorAll('.xterm')) {
    let ancestor = element.parentElement;
    for (let depth = 0; depth < 6 && ancestor; depth++, ancestor = ancestor.parentElement) {
      const key = Object.keys(ancestor).find(name => name.startsWith('__reactFiber$'));
      if (!key) continue;
      let ours = false;
      for (let fiber = ancestor[key], step = 0; fiber && step < 40; step++, fiber = fiber.return) {
        const props = fiber.memoizedProps;
        const terminalId = props?.terminalApi?.id ?? props?.mediator?.terminalApi?.id;
        if (terminalId != null) {
          if (terminalId !== id) break;
          ours = true;
        }
        if (ours && props?.connection?.sessionLogData) return props.connection.sessionLogData;
      }
      break;
    }
  }
  return null;
}

function limitedText(message) {
  return String(message || 'Unknown connection failure')
    .replace(/[\x00-\x08\x0b-\x1f\x7f]/g, ' ').slice(0, 1200);
}

function withTimeout(promise, ms, context) {
  let timeout;
  const deadline = new Promise((_, reject) => {
    timeout = setTimeout(() => reject(new Error(context + ' timed out after ' + Math.ceil(ms / 1000) + ' seconds')), ms);
  });
  return Promise.race([promise, deadline]).finally(() => clearTimeout(timeout));
}

export function createOriginalTerminalBridge({
  invokeNative,
  listenNative,
  store,
  doc = document,
  terminalOutput = null,          // libtermius terminalOutput (bridge/session-log.js)
}) {
  const active = new Map();       // original Termius terminal id -> native binding
  const starting = new Set();
  const failed = new Set();       // block automatic retry storms
  const logData = new Map();      // terminal id -> sessionLogData from PRE_CREATE_HISTORY
  const scanTimers = new Set();
  let listener = null;
  let disposed = false;
  let previousIds = '';
  let nativeFlowStyle = null;

  // The recovered renderer starts a second, Electron-only ConnectionController
  // on every SSH tab. It can remain on its Username/password form despite a
  // working native OpenSSH PTY, and intercepts clicks/keyboard input above xterm.
  // Hide only that legacy overlay, scoped to the corresponding native tab.
  function revealNativeTerminal(bound) {
    if (bound.protocol !== 'ssh' || bound.closed || (!bound.connected && !failed.has(bound.id))) return;
    const root = doc.getElementById?.(bound.id);
    if (!root?.querySelector?.('.xterm')) return;
    if (!nativeFlowStyle && doc.createElement && doc.head) {
      nativeFlowStyle = doc.createElement('style');
      nativeFlowStyle.textContent = '[data-termian-native-terminal="true"] > [class*="ConnectedConnectionFlow-root"] { display: none !important; pointer-events: none !important; }';
      doc.head.appendChild(nativeFlowStyle);
    }
    if (root.dataset) root.dataset.termianNativeTerminal = 'true';
    const state = store.getState().terminal;
    const activeTab = state?.tabs?.find(tab => tab.id === state.activeTabId);
    if (bound.connected && (activeTab?.terminalId === bound.id || state?.all?.[bound.id]?.isActive)) {
      bound.api?.terminal.focus?.();
    }
  }

  function dispatchTerminal(id, changes) {
    if (!store.getState().terminal?.all?.[id]) return;
    store.dispatch({type:'PATCH_TERMINAL',payload:{id,changes}});
  }

  function writeToTerminal(bound, data) {
    if (bound.closed) return;
    if (bound.api) {
      bound.api.terminal.write(data);
    } else {
      // Keep only the most recent ~256 KiB while Termius renders its original
      // connection-flow screen. Flush when its xterm mounts.
      const bytes = typeof data === 'string' ? Buffer.from(data, 'utf8') : Buffer.from(data);
      bound.pending.push(bytes);
      bound.pendingBytes += bytes.byteLength;
      while (bound.pendingBytes > 256 * 1024 && bound.pending.length > 1) {
        bound.pendingBytes -= bound.pending.shift().byteLength;
      }
    }
  }

  // Electron's node-pty process wrote local session logs; here the PTY output
  // passes through this bridge, so it writes them with the same Writer as SSH.
  function ensureLog(bound) {
    if (bound.log || bound.closed || !terminalOutput) return;
    const data = logData.get(bound.id) ?? findOriginalSessionLogData(bound.id, doc);
    if (typeof data?.name !== 'string' || typeof data.secretKey !== 'string') return;
    bound.log = new terminalOutput.Writer({
      filename: data.name,
      encryptionKey: Buffer.from(data.secretKey, 'base64'),
      finishedCallback: () => store.dispatch({type:'FINISH_SESSION_LOG_WRITING',payload:bound.id}),
    });
    for (const chunk of bound.unlogged) bound.log.append(chunk);
    bound.unlogged.length = 0;
    bound.unloggedBytes = 0;
  }

  function logOutput(bound, bytes) {
    if (!bound.log) {
      // Output can arrive before the history entry announces the log.
      bound.unlogged.push(bytes);
      bound.unloggedBytes += bytes.byteLength;
      while (bound.unloggedBytes > 1024 * 1024 && bound.unlogged.length > 1) {
        bound.unloggedBytes -= bound.unlogged.shift().byteLength;
      }
      ensureLog(bound);
      return;
    }
    bound.log.append(bytes);
  }

  function fail(bound, error) {
    if (bound.closed || failed.has(bound.id)) return;
    failed.add(bound.id);
    starting.delete(bound.id);
    bound.connected = false;
    bound.failure = limitedText(error?.message || error);
    bound.failedAt = Date.now();
    clearTimeout(bound.watchdog);
    writeToTerminal(bound, '\r\n\x1b[31m' + bound.failure + '\x1b[0m\r\n');
    // The original renderer renders the connection-flow page while this field
    // exists, even if connectionProcess is already 'error'. Clear the flow so
    // the original xterm can mount and display the native failure.
    dispatchTerminal(bound.id, {
      connectionEstablished:false,
      connectionProcess:'error',
      connectionFlow:undefined,
      disconnectTimestamp:bound.failedAt,
      isDisconnected:true,
    });
    revealNativeTerminal(bound);
    scheduleScan();
  }

  function attach(bound, api) {
    if (bound.closed || bound.api === api) return;
    for (const d of bound.disposables) try { d?.dispose?.(); } catch {}
    bound.disposables.length = 0;
    bound.api = api;
    bound.disposables.push(api.terminal.onData(data => {
      void invokeNative('write_terminal',{sessionId:bound.nativeSessionId,data})
        .catch(error => {
          if (bound.connected) console.error('[Termian] Native terminal input:',limitedText(error));
        });
    }));
    if (typeof api.terminal.onResize === 'function') {
      bound.disposables.push(api.terminal.onResize(size => {
        if (size?.cols >= 40 && size?.rows >= 10 && !bound.closed) {
          void invokeNative('resize_terminal',{
            sessionId:bound.nativeSessionId,cols:size.cols,rows:size.rows,
          }).catch(() => {});
        }
      }));
    }
    for (const data of bound.pending) {
      api.terminal.write(new Uint8Array(data));
    }
    bound.pending.length = 0;
    bound.pendingBytes = 0;
    ensureLog(bound);
    revealNativeTerminal(bound);
  }

  async function ensureEventListener() {
    if (!listener) {
      listener = Promise.resolve(listenNative('termian-terminal', event => {
        const payload = event.payload;
        if (!payload || typeof payload.sessionId !== 'string') return;
        // Each reconnect gets a unique native session id. Late events from
        // cancelled sessions cannot overwrite the new tab's Redux state.
        const bound = [...active.values()].find(x => x.nativeSessionId === payload.sessionId);
        if (!bound || bound.closed) return;
        if (payload.kind === 'output') {
          try {
            const bytes = Buffer.from(payload.data, 'base64');
            writeToTerminal(bound, bytes);
            logOutput(bound, bytes);
          } catch (error) { console.error('[Termian] PTY output decode:',limitedText(error)); }
        } else if (payload.kind === 'status') {
          if (payload.data === 'connected') {
            if (failed.has(bound.id)) return;
            bound.connected = true;
            clearTimeout(bound.watchdog);
            // Termius keeps the spinner visible until connectionFlow is
            // removed. The old Electron controller cannot finish that flow
            // for a session started by Tauri.
            dispatchTerminal(bound.id, {
              connectionEstablished:true,connectionProcess:'succeeded',
              connectionFlow:undefined,
              disconnectTimestamp:null,isDisconnected:false,
            });
            revealNativeTerminal(bound);
            scheduleScan();
          } else if (!failed.has(bound.id)) {
            fail(bound, 'SSH or terminal session was disconnected');
          }
        } else if (payload.kind === 'error') {
          fail(bound, payload.data);
        }
      }));
    }
    return listener;
  }

  async function begin(id, api = null) {
    if (disposed || active.has(id) || starting.has(id) || failed.has(id)) return;
    const terminal = store.getState().terminal?.all?.[id];
    if (!terminal || terminal.connectionProtocol !== 'local') return;

    starting.add(id);
    const bound = {
      id, protocol:terminal.connectionProtocol, nativeSessionId:crypto.randomUUID(), api:null,
      pending:[],pendingBytes:0,connected:false,closed:false,disposables:[],watchdog:null,
      log:null,unlogged:[],unloggedBytes:0,
    };
    active.set(id,bound);
    if (api) attach(bound,api);

    try {
      await withTimeout(ensureEventListener(),10000,'Subscribing to native terminal events');
      if (bound.closed) return;
      const currentApi = bound.api;
      const size = {
        cols:Math.min(400,Math.max(40,Number(currentApi?.terminal.cols)||80)),
        rows:Math.min(160,Math.max(10,Number(currentApi?.terminal.rows)||24)),
      };
      await withTimeout(invokeNative('start_terminal',{
        sessionId:bound.nativeSessionId,
        // Settings → Terminal → Local Terminal Path (null = platform default).
        connection:{serial:null,shell:store.getState().localTerminalPath?.path || null},
        size,
      }),12000,'Starting native terminal session');
      if (bound.closed) {
        void invokeNative('stop_terminal',{sessionId:bound.nativeSessionId}).catch(()=>{});
        return;
      }
      // start_terminal launches a Rust worker. Only its real 'connected'
      // status means authentication and shell/PTY opening succeeded.
      if (!bound.connected && !failed.has(id)) {
        bound.watchdog = setTimeout(() => {
          fail(bound,'Local terminal did not start within 30 seconds');
          void invokeNative('stop_terminal',{sessionId:bound.nativeSessionId}).catch(()=>{});
        },30000);
      }
    } catch (error) {
      fail(bound,error);
      console.error('[Termian] Native terminal start failed:',limitedText(error?.message || error));
    } finally {
      starting.delete(id);
    }
  }

  function closeBound(id) {
    const bound = active.get(id);
    if (!bound) return;
    bound.closed = true;
    const root = doc.getElementById?.(id);
    if (root?.dataset) delete root.dataset.termianNativeTerminal;
    clearTimeout(bound.watchdog);
    for (const d of bound.disposables) try { d?.dispose?.(); } catch {}
    bound.disposables.length = 0;
    bound.log?.end();
    logData.delete(id);
    active.delete(id);
    void invokeNative('stop_terminal',{sessionId:bound.nativeSessionId}).catch(()=>{});
  }

  function scan() {
    if (disposed) return;
    const terminals = store.getState().terminal?.all || {};
    for (const [id,t] of Object.entries(terminals)) {
      const bound = active.get(id);
      if (bound) {
        if (!bound.api) {
          const api = findOriginalTerminalApi(id,doc);
          if (api) attach(bound,api);
        }
        continue;
      }
      if (t.connectionProtocol !== 'local' || starting.has(id) || failed.has(id)) continue;
      const api = findOriginalTerminalApi(id,doc);
      if (api) void begin(id,api);
    }
    for (const id of [...active.keys()]) {
      if (!(id in terminals)) closeBound(id);
    }
    for (const id of failed) {
      if (!(id in terminals)) failed.delete(id);
    }
  }

  function scheduleScan() {
    if (disposed) return;
    // React may mount xterm only after Redux switches away from the original
    // SSH connection flow. Retry lookup without triggering a new SSH session.
    for (const ms of [15,80,180,450,900,1600]) {
      const timer=setTimeout(()=>{scanTimers.delete(timer);scan();},ms);
      scanTimers.add(timer);
    }
  }

  let reconciling = false;
  function reconcileConnectedState() {
    if (reconciling || disposed) return;
    reconciling = true;
    try {
      for (const [id,bound] of active) {
        const isFailed = failed.has(id);
        if (!bound.connected && !isFailed) continue;
        const t = store.getState().terminal?.all?.[id];
        if (!t) continue;
        const changes = isFailed ? {
          connectionEstablished:false,connectionProcess:'error',
          connectionFlow:undefined,isDisconnected:true,
        } : {
          connectionEstablished:true,connectionProcess:'succeeded',
          connectionFlow:undefined,isDisconnected:false,disconnectTimestamp:null,
        };
        if (Object.entries(changes).some(([key,value]) => t[key] !== value) ||
            (isFailed && t.disconnectTimestamp == null)) {
          if (isFailed) changes.disconnectTimestamp = bound.failedAt || Date.now();
          dispatchTerminal(id,changes);
        }
      }
    } finally { reconciling=false; }
  }

  function onAction(action) {
    if (disposed) return;
    if (action.type === 'PRE_CREATE_HISTORY') {
      const { connection, terminalId, errorMessage } = action.payload ?? {};
      if (connection?.protocol === 'local' && connection.sessionLogData && !errorMessage) {
        logData.set(terminalId, connection.sessionLogData);
        const bound = active.get(terminalId);
        if (bound) ensureLog(bound);
      }
    }
    // Legacy Electron sagas can still issue INIT_CONNECTION, REQUEST_NEW_STEP
    // and other delayed status updates. Never allow those actions to restore
    // the spinner or conceal an actual Tauri connection error.
    reconcileConnectedState();

    // Explicit reconnect may target a former failed session. A new unique
    // native ID prevents old worker events from overwriting its status.
    if (action.type === 'RECONNECT_TAB' || action.type === 'RECONNECT_TERMINAL') {
      for (const id of failed) {
        closeBound(id);
      }
      failed.clear();
      scheduleScan();
      return;
    }

    const ids=Object.keys(store.getState().terminal?.all || {}).sort().join('|');
    if (ids !== previousIds || WAKE_ACTIONS.has(action.type) ||
        action.type.includes('CLOSE_TAB') || action.type.includes('REMOVE_TERMINAL')) {
      previousIds=ids;
      scheduleScan();
    }
  }

  return {
    onAction,scan,active,failed,reconcileConnectedState,
    async dispose(){
      disposed=true;
      for(const timer of scanTimers)clearTimeout(timer);
      scanTimers.clear();
      for(const id of [...active.keys()])closeBound(id);
      failed.clear();
      if(listener) await listener.then(unlisten => unlisten()).catch(()=>{});
      listener=null;
      nativeFlowStyle?.remove?.();
      nativeFlowStyle=null;
    },
  };
}
