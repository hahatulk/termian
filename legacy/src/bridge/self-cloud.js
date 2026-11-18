// Self-hosted, encrypted snapshot sync. The UI talks to the existing background
// Store, while config/secrets and the network stay in Rust. Polling is
// read-only; "sync" (button or hourly, unless disabled) merges both sides
// with bridge/sync-merge.js, and upload/download replace a whole snapshot.
import ui from '../self-cloud-ui.cjs';
import adapter from '../self-cloud-adapter.cjs';
import {
  indexSnapshot, serverKey, readBaseline, saveBaseline, compareSnapshots,
} from './sync-diff.js';
import {
  toSyncForm, digestsOf, readMergeState, saveMergeState, syncOnce, settleImport,
} from './sync-merge.js';

const REQUEST_TIMEOUT = 5 * 60_000;
const POLL_INTERVAL = 60_000;
const AUTO_SYNC_FIRST = 60_000;
const AUTO_SYNC_INTERVAL = 60 * 60_000;
const LAST_SYNC = 'termian:self-cloud:last-sync';
// src/rust/native.rs REVISION_CHANGED: another device uploaded meanwhile.
const REVISION_CHANGED = 'Снимок в облаке изменился во время синхронизации';
// Wrong token / passphrase errors from src/rust/native.rs. Background polling
// must not keep retrying them: every attempt extends the server lockout or
// spends the local wrong-passphrase allowance.
const CREDENTIAL_ERROR = /токен доступа|отклонил авторизацию|неверных попыток авторизации|неверный пароль шифрования|неверных паролей/i;
export const isCredentialError = message => CREDENTIAL_ERROR.test(String(message || ''));

export function installSelfCloud({ invoke, bus, isBackground, isPrimary = false }) {
  const pending = new Map();
  const subscribers = new Set();
  let backgroundReady = false;
  let queue = Promise.resolve();
  let inFlight = null;
  let latest = Object.freeze({kind: 'checking', changes: [], total: 0});
  let remoteCache = null; // only derived SHA-256 indexes, never decrypted records
  let credentialsRejected = false; // pauses automatic checks until the user acts

  function update(next) {
    latest = Object.freeze(next);
    for (const listener of subscribers) {
      try { listener(latest); }
      catch (error) { console.error('[Self cloud] Status listener failed', error); }
    }
    return latest;
  }

  if (!isBackground) {
    bus.on('self-cloud:register', () => {
      backgroundReady = true;
      if (isPrimary) void autoCheck();
    });
    bus.on('self-cloud:response', (_event, message) => {
      const request = pending.get(message?.id);
      if (!request) return;
      clearTimeout(request.timer);
      pending.delete(message.id);
      if (message.ok) request.resolve(message.result ?? message.snapshot);
      else request.reject(new Error(String(message.error || 'Операция не выполнена').slice(0, 240)));
    });
  }

  function requestBackground(operation, payload, timeoutMs = REQUEST_TIMEOUT) {
    const id = crypto.randomUUID();
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        pending.delete(id);
        reject(new Error(backgroundReady ? 'Фоновый адаптер не ответил' : 'Фоновый адаптер недоступен'));
      }, timeoutMs);
      pending.set(id, {resolve, reject, timer});
      bus.send('self-cloud:request', {id, operation, ...(payload ?? {})});
    });
  }

  const enqueue = work => {
    const run = queue.catch(() => {}).then(work);
    queue = run.catch(() => {});
    return run;
  };
  const failure = error => ({
    ok: false, error: String(error?.message || error || 'Ошибка синхронизации').slice(0, 240),
  });

  async function poll() {
    update({...latest, kind: backgroundReady ? 'checking' : 'waiting'});
    try {
      const metadata = await invoke('self_cloud_get_config');
      if (!metadata?.configured) {
        remoteCache = null;
        return update({kind: 'unconfigured', changes: [], total: 0});
      }
      const key = await serverKey(metadata.url);
      const probe = await invoke('self_cloud_probe');
      let remote;
      if (!probe?.exists) {
        remote = new Map();
        remoteCache = {key, exists: false, revision: null, entries: remote};
      } else if (remoteCache?.key === key &&
                 remoteCache.exists && probe.revision &&
                 remoteCache.revision === probe.revision) {
        remote = remoteCache.entries;
      } else {
        const snapshot = await invoke('self_cloud_download');
        remote = await indexSnapshot(snapshot);
        remoteCache = {key, exists: true, revision: probe.revision, entries: remote};
        // Decrypted snapshot is no longer retained once its safe index is built.
      }
      const local = await indexSnapshot(await requestBackground('export', undefined, 12_000));
      backgroundReady = true;
      const baseline = readBaseline(localStorage, key);
      const result = {...compareSnapshots(local, remote, baseline, !!probe.exists), ...lastSync()};
      if (result.kind === 'synced' && baseline !== null) {
        // Both stores now match: acknowledge converged snapshots.
        saveBaseline(localStorage, key, local, remote);
      }
      return update(result);
    } catch (error) {
      const message = String(error?.message || error || 'Ошибка проверки облака').slice(0, 240);
      credentialsRejected = isCredentialError(message);
      return update({
        kind: 'error', changes: [], total: 0, error: message,
        credentials: credentialsRejected, checkedAt: Date.now(),
      });
    }
  }
  function check() {
    if (inFlight) return inFlight;
    inFlight = enqueue(poll).finally(() => { inFlight = null; });
    return inFlight;
  }
  // Timers, focus and window registration: skipped after a credential error.
  function autoCheck() {
    if (credentialsRejected) return Promise.resolve(latest);
    return check();
  }

  function lastSync() {
    const syncedAt = Number(localStorage.getItem(LAST_SYNC)) || null;
    return syncedAt ? {syncedAt} : {};
  }
  // The importer and the adapter re-check the reference graph; never send a
  // snapshot that the other devices could not import.
  const importable = snapshot => ({...snapshot, ...adapter.validateSnapshot(snapshot)});

  async function remember(key, { ids, exported, local, remote, digests = null }) {
    saveMergeState(localStorage, key, digests ?? {ids, local: await digestsOf(local), remote: await digestsOf(remote)});
    // Anchor both sides: the imported data re-exports with importer defaults.
    saveBaseline(localStorage, key, await indexSnapshot(exported), await indexSnapshot(remote));
    localStorage.setItem(LAST_SYNC, String(Date.now()));
    remoteCache = null;
  }

  const exportLocal = () => requestBackground('export');
  const importLocal = async snapshot => (await requestBackground('import', {snapshot}))?.ids;

  async function mergeOnce(key, prefer) {
    const saved = readMergeState(localStorage, key);
    const result = await syncOnce({
      saved, prefer, exportLocal, importLocal,
      fetchRemote: () => invoke('self_cloud_fetch'),
      upload: (snapshot, { revision, create }) => invoke('self_cloud_upload', {snapshot, revision, create}),
      backup: snapshot => invoke('self_cloud_backup', {snapshot}),
      validate: importable,
    });
    if (result.conflicts.length) {
      if (result.state) saveMergeState(localStorage, key, result.state);
      return result;
    }
    await remember(key, {exported: result.exported, remote: result.remote, digests: result.state});
    return result;
  }

  async function mergeSync(key, prefer) {
    update({...latest, kind: 'syncing'});
    for (let attempt = 0; ; attempt++) {
      try {
        return await mergeOnce(key, prefer);
      } catch (error) {
        if (attempt < 2 && String(error?.message || error).includes(REVISION_CHANGED)) continue;
        throw error;
      }
    }
  }

  async function action(options) {
    const operation = options?.operation;
    const keys = Object.keys(options ?? {});
    const valid = ['upload', 'download'].includes(operation) ? keys.length === 1
      : operation === 'sync' && keys.every(name => ['operation', 'prefer'].includes(name)) &&
        [undefined, 'local', 'remote'].includes(options.prefer);
    if (!valid) throw new Error('Недопустимая операция');
    const metadata = await invoke('self_cloud_get_config');
    if (!metadata?.configured) throw new Error('Сначала настройте своё облако');
    const key = await serverKey(metadata.url);
    if (operation === 'sync') {
      const result = await mergeSync(key, options.prefer ?? null);
      if (result.conflicts.length) {
        update({kind: 'conflict', changes: result.conflicts.slice(0, 100), total: result.conflicts.length,
          upload: 0, download: 0, conflicts: result.conflicts.length, unknown: 0, checkedAt: Date.now(), ...lastSync()});
        return {ok: false, conflicts: result.conflicts,
          error: 'Одни и те же записи изменены и на устройстве, и в облаке. Выберите, чью версию оставить.'};
      }
      update({kind: 'synced', changes: [], total: 0, checkedAt: Date.now(), ...lastSync()});
      const parts = [];
      if (result.fromLocal) parts.push('отправлено изменений: ' + result.fromLocal);
      if (result.fromRemote) parts.push('получено изменений: ' + result.fromRemote);
      return {ok: true, status: parts.length ? 'Синхронизировано: ' + parts.join(', ') : 'Данные уже синхронизированы'};
    }
    const saved = readMergeState(localStorage, key);
    if (operation === 'upload') {
      // Replace the cloud snapshot, keeping the stable sync IDs.
      const ids = saved?.ids ?? {};
      const exported = await exportLocal();
      const local = toSyncForm(exported, ids);
      const snapshot = importable(local);
      await invoke('self_cloud_upload', {snapshot});
      await remember(key, {ids, exported, local, remote: snapshot});
      update({kind: 'synced', changes: [], total: 0, checkedAt: Date.now(), ...lastSync()});
      return {ok: true, status: 'Данные загружены'};
    }
    const snapshot = await invoke('self_cloud_download');
    await invoke('self_cloud_backup', {snapshot: await exportLocal()});
    const settled = await settleImport(await importLocal(snapshot), snapshot, exportLocal);
    await remember(key, {...settled, remote: snapshot});
    update({kind: 'synced', changes: [], total: 0, checkedAt: Date.now(), ...lastSync()});
    return {ok: true, status: 'Данные загружены и применены'};
  }

  // Settings → «Логи сессий»: since = start of the period, 0 = everything.
  async function clearLogs(options) {
    const since = options?.since;
    if (!Number.isSafeInteger(since) || since < 0) throw new Error('Недопустимый период');
    const result = await requestBackground('clear-logs', {since}, 2 * 60_000);
    const native = await invoke('session_logs_clear', {since, keep: result?.kept ?? []});
    return {ok: true, entries: result?.entries ?? 0, files: native?.local ?? 0, remote: native?.remote ?? null};
  }

  // Hourly merge sync (primary window). Conflicts are left for the user.
  async function autoSync() {
    if (credentialsRejected) return;
    const metadata = await invoke('self_cloud_get_config').catch(() => null);
    if (!metadata?.configured || metadata.autoSync === false) return;
    const result = await enqueue(() => action({operation: 'sync'})).catch(failure);
    if (result?.ok === false && !result.conflicts) {
      credentialsRejected = isCredentialError(result.error);
      update({kind: 'error', changes: [], total: 0, error: result.error,
        credentials: credentialsRejected, checkedAt: Date.now(), ...lastSync()});
    } else if (result?.ok) void check();
  }

  const ipc = {
    invoke(channel, options) {
      switch (channel) {
        case 'self-cloud:get-config':
          return invoke('self_cloud_get_config');
        case 'self-cloud:status':
          return Promise.resolve(latest);
        case 'self-cloud:check':
          return check();
        case 'self-cloud:save-config':
          return enqueue(() => invoke('self_cloud_save_config', {config: options ?? {}}))
            .then(config => {
              remoteCache = null;
              credentialsRejected = false;
              void check();
              return {ok: true, ...config};
            }, failure);
        case 'self-cloud:action':
          return enqueue(() => action(options)).then(result => {
            if (result?.ok !== false) void check(); // fresh status after a user-confirmed operation
            return result;
          }, error => {
            const result = failure(error);
            update({kind: 'error', changes: [], total: 0, error: result.error,
              credentials: isCredentialError(result.error), checkedAt: Date.now(), ...lastSync()});
            return result;
          });
        case 'self-cloud:set-auto-sync':
          return invoke('self_cloud_set_auto_sync', {enabled: options?.enabled !== false});
        case 'self-cloud:clear-logs':
          return clearLogs(options).catch(failure);
        default:
          return Promise.reject(new Error('Unsupported self-cloud channel: ' + channel));
      }
    },
  };

  if (isPrimary) {
    setTimeout(() => void autoCheck(), 1800);
    setTimeout(() => void autoSync(), AUTO_SYNC_FIRST);
    const syncTimer = setInterval(() => void autoSync(), AUTO_SYNC_INTERVAL);
    window.addEventListener('beforeunload', () => clearInterval(syncTimer), {once: true});
    const timer = setInterval(() => {
      if (!document.hidden) void autoCheck();
    }, POLL_INTERVAL);
    window.addEventListener('beforeunload', () => clearInterval(timer), {once: true});
    document.addEventListener('visibilitychange', () => {
      if (!document.hidden) void autoCheck();
    });
    window.addEventListener('focus', () => void autoCheck());
  }
  return {
    invoke: (channel, ...args) => ipc.invoke(channel, ...args),
    api: Object.freeze({
      open: operation => ui.open(ipc, operation),
      action: options => ipc.invoke('self-cloud:action', options),
      sync: prefer => ipc.invoke('self-cloud:action', prefer ? {operation: 'sync', prefer} : {operation: 'sync'}),
      setAutoSync: enabled => ipc.invoke('self-cloud:set-auto-sync', {enabled}),
      clearLogs: since => ipc.invoke('self-cloud:clear-logs', {since}),
      status: () => latest,
      check,
      config: () => ipc.invoke('self-cloud:get-config'),
      subscribe(listener) {
        subscribers.add(listener);
        return () => subscribers.delete(listener);
      },
    }),
  };
}
