// Renderer-to-renderer ipcRenderer channels (Electron routed them through the
// main process). Handlers receive (event, ...args) like Electron listeners.
const CHANNEL = 'termian:ipc';

export function createIpcBus({ source }) {
  const channel = new BroadcastChannel(CHANNEL);
  const listeners = new Map();

  channel.addEventListener('message', ({data}) => {
    if (!data || data.source === source || typeof data.channel !== 'string') return;
    const event = {sender: {id: data.source, send: (name, ...args) => bus.send(name, ...args)}};
    for (const handler of [...(listeners.get(data.channel) ?? [])]) {
      try { handler(event, ...(data.args ?? [])); }
      catch (error) { console.error('[Termian] IPC handler failed for', data.channel, error); }
    }
  });

  const bus = {
    on(name, handler) {
      if (!listeners.has(name)) listeners.set(name, new Set());
      listeners.get(name).add(handler);
    },
    once(name, handler) {
      const wrapped = (...args) => { bus.off(name, wrapped); handler(...args); };
      bus.on(name, wrapped);
    },
    off(name, handler) { listeners.get(name)?.delete(handler); },
    offAll(name) { listeners.delete(name); },
    send(name, ...args) {
      try { channel.postMessage({source, channel: name, args}); }
      catch (error) { console.error('[Termian] IPC send failed for', name, error); }
    },
    invoke(name) {
      return Promise.reject(new Error('Unsupported legacy IPC channel: ' + name));
    },
  };
  return bus;
}
