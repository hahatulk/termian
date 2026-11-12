// Notice for an update the native updater (src/rust/updater.rs) has already
// downloaded, verified and installed. It only takes effect on restart, which
// the user triggers, so open SSH sessions are never cut off by an update.
const STYLE = `
#termian-update{position:fixed;right:16px;bottom:16px;z-index:2147483645;max-width:360px;display:flex;flex-direction:column;gap:10px;padding:14px 16px;border:1px solid #454950;border-radius:10px;background:#272b30;box-shadow:0 12px 40px #0006;color:#f2f3f5;font:13px/1.45 system-ui,sans-serif;-webkit-app-region:no-drag}
#termian-update p{margin:0;color:#9ea5b3;font-size:12px}
#termian-update strong{font-weight:600}
#termian-update .tu-actions{display:flex;justify-content:flex-end;gap:8px}
#termian-update button{min-height:28px;padding:4px 12px;border:1px solid #454950;border-radius:6px;background:#35393f;color:#f2f3f5;font:inherit;font-size:12px;cursor:pointer}
#termian-update button:hover{background:#454950}
#termian-update button[data-restart]{border-color:#3d7bfd;background:#3d7bfd}
#termian-update button[data-restart]:hover{background:#5a8fff}
`;

export function installUpdateNotice({ invoke, listen, doc = document }) {
  let dismissed = false;
  let notice = null;

  function show(status) {
    if (dismissed || notice || status?.state !== 'installed') return;
    // The bridge runs from <head>, before the body exists.
    if (!doc.body) return void doc.addEventListener('DOMContentLoaded', () => show(status), {once: true});
    if (!doc.getElementById('termian-update-style')) {
      const style = doc.createElement('style');
      style.id = 'termian-update-style';
      style.textContent = STYLE;
      doc.head.appendChild(style);
    }
    notice = doc.createElement('div');
    notice.id = 'termian-update';
    notice.setAttribute('role', 'status');
    const title = doc.createElement('strong');
    title.textContent = 'Termian ' + (status.version || '') + ' установлен';
    const text = doc.createElement('p');
    text.textContent = 'Новая версия запустится после перезапуска. Открытые SSH-сессии при перезапуске закроются.';
    const actions = doc.createElement('div');
    actions.className = 'tu-actions';
    const later = doc.createElement('button');
    later.type = 'button';
    later.textContent = 'Позже';
    later.onclick = () => { dismissed = true; notice.remove(); notice = null; };
    const restart = doc.createElement('button');
    restart.type = 'button';
    restart.dataset.restart = '';
    restart.textContent = 'Перезапустить';
    restart.onclick = () => {
      restart.disabled = true;
      invoke('update_restart').catch(error => {
        restart.disabled = false;
        console.error('[Termian] Restart for update failed:', error);
      });
    };
    actions.append(later, restart);
    notice.append(title, text, actions);
    doc.body.appendChild(notice);
  }

  void listen('termian://update-status', event => show(event.payload))
    .catch(error => console.error('[Termian] Update status listener failed:', error));
  // The update may have been installed before this page loaded.
  void invoke('update_status').then(show)
    .catch(error => console.error('[Termian] Update status failed:', error));
}

// window.api.updates for Settings → Version: current status, a manual check
// with download progress, and restart once the update is installed.
export function createUpdatesApi({ invoke, listen }) {
  let latest = {state: 'idle'};
  const subscribers = new Set();
  const publish = status => {
    if (!status?.state) return;
    latest = status;
    for (const listener of subscribers) {
      try { listener(latest); } catch (error) { console.error('[Termian] Update listener failed:', error); }
    }
  };
  void listen('termian://update-status', event => publish(event.payload))
    .catch(error => console.error('[Termian] Update status listener failed:', error));
  void invoke('update_status').then(publish).catch(() => {});
  return Object.freeze({
    status: () => latest,
    subscribe(listener) {
      subscribers.add(listener);
      return () => subscribers.delete(listener);
    },
    // Resolves with the status at the start; progress arrives through subscribe().
    check: () => invoke('update_check').then(status => { publish(status); return status; }),
    restart: () => invoke('update_restart'),
  });
}
