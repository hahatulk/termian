// Replacement for electron-redux. In Electron every renderer (UI window,
// settings window, hidden background window) forwarded its actions to the main
// process, which re-broadcast them to all renderers with meta.scope = 'local'.
// All Tauri windows share one origin, so a BroadcastChannel provides the same
// fan-out without a privileged hub. Each window reduces its own copy first.
const CHANNEL = 'termian:redux';

function isForwardable(action) {
  return action && typeof action.type === 'string' &&
    !action.type.startsWith('@@') && !action.type.startsWith('redux-form') &&
    action.meta?.scope !== 'local';
}

// Electron's IPC refused non-cloneable payloads; keep the action flowing locally
// and send a copy without functions/symbols instead of throwing. Never fall
// back to JSON: the background's lifecycle/REHYDRATE carries Map state
// (sidepanel.windows) and the UI selectors call .get() on it.
function withoutFunctions(value, seen = new WeakMap()) {
  if (typeof value === 'function' || typeof value === 'symbol') return undefined;
  if (value === null || typeof value !== 'object') return value;
  if (seen.has(value)) return seen.get(value);
  if (value instanceof Map) {
    const copy = new Map();
    seen.set(value, copy);
    for (const [key, item] of value) {
      const clean = withoutFunctions(item, seen);
      if (clean !== undefined) copy.set(withoutFunctions(key, seen), clean);
    }
    return copy;
  }
  if (value instanceof Set) {
    const copy = new Set();
    seen.set(value, copy);
    for (const item of value) {
      const clean = withoutFunctions(item, seen);
      if (clean !== undefined) copy.add(clean);
    }
    return copy;
  }
  if (Array.isArray(value)) {
    const copy = [];
    seen.set(value, copy);
    for (const item of value) copy.push(withoutFunctions(item, seen) ?? null);
    return copy;
  }
  // Dates, binary data, errors and similar are cloned natively.
  try { return structuredClone(value); } catch { /* has functions inside */ }
  const copy = {};
  seen.set(value, copy);
  for (const [key, item] of Object.entries(value)) {
    const clean = withoutFunctions(item, seen);
    if (clean !== undefined) copy[key] = clean;
  }
  return copy;
}

function cloneable(message) {
  try {
    structuredClone(message);
    return message;
  } catch {
    return withoutFunctions(message);
  }
}

export function createReduxRelay({ source, onAction = () => {} }) {
  const channel = new BroadcastChannel(CHANNEL);
  let store = null;
  const queued = [];

  channel.addEventListener('message', event => {
    const message = event.data;
    if (message?.kind !== 'action' || message.source === source) return;
    if (store) store.dispatch(message.action);
    else queued.push(message.action);
  });

  return {
    forwardToMain: () => next => action => {
      if (!isForwardable(action)) {
        const result = next(action);
        onAction(action);
        return result;
      }
      const local = {...action, meta: {...action.meta, scope: 'local'}};
      try {
        channel.postMessage(cloneable({kind: 'action', source, action: local}));
      } catch (error) {
        console.error('[Termian] Redux relay failed for', action.type, error);
      }
      const result = next(local);
      onAction(local);
      return result;
    },
    replayActionRenderer(target) {
      store = target;
      for (const action of queued.splice(0)) store.dispatch(action);
    },
    close() { channel.close(); },
  };
}
