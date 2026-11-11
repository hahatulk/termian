// Silent backup of terminal session logs to the self-hosted cloud. Unlike the
// vault snapshot (bridge/self-cloud.js) it runs on its own: it never updates
// the sync status, the baseline or the upload/download badges in the header,
// and it never downloads into the vault. Rust uploads only new or changed
// log files, sealed with the self-cloud passphrase. It follows the hourly
// auto-sync switch in Settings (self-cloud `autoSync`).
const FIRST_RUN = 45_000;
const INTERVAL = 60 * 60_000;
const AFTER_SESSION = 5_000;
const CREDENTIAL_PAUSE = 60 * 60_000;

// History entries (Logs screen) with a log file. `meta` is what a restored
// copy needs; credentials stored on the entry are deliberately left out.
export function sessionLogItems(state) {
  const histories = state?.entities?.histories;
  const list = Array.isArray(histories) ? histories : Object.values(histories ?? {});
  const items = new Map();
  for (const history of list) {
    const data = history?.session_log_data ?? history?.command?.session_log_data;
    if (typeof data?.name !== 'string' || typeof data.secretKey !== 'string') continue;
    items.set(data.name, {
      name: data.name,
      meta: {
        local_id: history.local_id ?? null,
        label: history.label || '',
        address: history.address || '',
        port: history.port || 0,
        username: history.username || '',
        protocol: history.protocol ?? null,
        connected_at: history.connected_at ?? null,
        disconnectedAt: history.disconnectedAt ?? null,
        session_log_data: {name: data.name, secretKey: data.secretKey},
      },
    });
  }
  return [...items.values()];
}

// `isCredentialError` comes from bridge/self-cloud.js (wrong token/passphrase).
export function installSessionLogSync({ invoke, getState, isCredentialError = () => false, timers = globalThis }) {
  let timer = null;
  let running = null;
  let pausedUntil = 0;
  let unsupported = false;

  async function run() {
    if (unsupported || Date.now() < pausedUntil) return null;
    const items = sessionLogItems(getState());
    if (!items.length) return null;
    try {
      const config = await invoke('self_cloud_get_config');
      if (!config?.configured || config.autoSync === false) return null;
      const result = await invoke('session_logs_cloud_sync', {items});
      // An older sync server without the log API: stop until restart.
      if (result?.unsupported) unsupported = true;
      return result;
    } catch (error) {
      const message = String(error?.message || error);
      // Do not extend a server lockout with background retries.
      if (isCredentialError(message)) pausedUntil = Date.now() + CREDENTIAL_PAUSE;
      console.warn('[Termian] Session log backup failed:', message.slice(0, 240));
      return null;
    }
  }

  function schedule(delay) {
    timers.clearTimeout(timer);
    timer = timers.setTimeout(() => {
      timer = null;
      if (running) return void schedule(AFTER_SESSION);
      running = run().finally(() => { running = null; });
    }, delay);
  }

  schedule(FIRST_RUN);
  const interval = timers.setInterval(() => schedule(0), INTERVAL);
  return {
    run,
    // A session ended: its log is complete, back it up shortly.
    sessionFinished: () => schedule(AFTER_SESSION),
    dispose() {
      timers.clearTimeout(timer);
      timers.clearInterval(interval);
    },
  };
}
