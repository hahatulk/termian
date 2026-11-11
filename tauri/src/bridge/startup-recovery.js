// First-run safeguard for the primary Tauri WebView. A fresh WebView profile
// has no persisted readiness marker; an existing profile should not show the
// preparation screen on every normal launch. Never delete or reset the vault.
const RELOAD_KEY = 'termian:startup:reload-attempted';
const PROFILE_READY_KEY = 'termian:startup:profile-ready';

export function installStartupRecovery({
  hostWindow = window,
  doc = hostWindow.document,
  overlayDelayMs = 650,
  timeoutMs = 10000,
  intervalMs = 2000,
} = {}) {
  let finished = false;
  let overlay = null;
  let observer = null;
  let interval = null;
  let showTimer = null;
  let recoveryTimer = null;
  let root = null;
  // Keep the first startup errors visible in the failure overlay: a release
  // WebView has no developer tools, and a silent retry hides the real cause.
  const startupErrors = [];

  function rememberError(text) {
    if (finished || startupErrors.length >= 5) return;
    startupErrors.push(String(text).slice(0, 600));
  }

  function describeReason(reason) {
    // WebKit stacks omit the message, so always keep both.
    if (reason instanceof Error) return [`${reason.name}: ${reason.message}`, reason.stack].filter(Boolean).join('\n');
    if (reason && typeof reason === 'object') {
      try { return JSON.stringify(reason); } catch { /* fall through */ }
    }
    return String(reason);
  }

  function clearTimers() {
    if (interval !== null) hostWindow.clearInterval(interval);
    if (showTimer !== null) hostWindow.clearTimeout(showTimer);
    if (recoveryTimer !== null) hostWindow.clearTimeout(recoveryTimer);
    observer?.disconnect();
    observer = null;
    interval = showTimer = recoveryTimer = null;
  }

  function safelyRemember(key, value) {
    try {
      if (value === null) hostWindow.sessionStorage.removeItem(key);
      else hostWindow.sessionStorage.setItem(key, value);
    } catch {
      // Private WebViews may disable sessionStorage. Recovery is best-effort.
    }
  }

  function hasReloaded() {
    try { return hostWindow.sessionStorage.getItem(RELOAD_KEY) === '1'; }
    catch { return true; } // Never risk an infinite reload loop.
  }

  function profileIsReady() {
    try { return hostWindow.localStorage.getItem(PROFILE_READY_KEY) === '1'; }
    catch { return true; } // Without persistent storage, never claim a fresh profile.
  }

  function rememberReadyProfile() {
    try { hostWindow.localStorage.setItem(PROFILE_READY_KEY, '1'); }
    catch { /* Read-only WebViews still get normal failure recovery. */ }
  }

  function hasRenderedUi() {
    // An empty React root or a mounted but blank shell is not a usable first
    // launch. Check visible UI text, not only the presence of a React node.
    if (!root || !root.firstElementChild) return false;
    const visibleText = root.innerText ?? root.textContent ?? '';
    return visibleText.trim().length > 0;
  }

  function finish() {
    if (finished) return;
    finished = true;
    rememberReadyProfile(); // Commit only after the real UI has rendered.
    clearTimers();
    overlay?.remove();
    overlay = null;
    safelyRemember(RELOAD_KEY, null);
  }

  function displayOverlay(failed = false) {
    if (finished || !doc.body || !root || hasRenderedUi() || (!failed && profileIsReady())) return;
    if (!overlay) {
      overlay = doc.createElement('div');
      overlay.id = 'termian-startup-status';
      Object.assign(overlay.style, {
        position: 'fixed',
        inset: '0',
        zIndex: '2147483647',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        gap: '16px',
        background: '#131626',
        color: '#f4f5f8',
        fontFamily: 'system-ui, sans-serif',
        textAlign: 'center',
        padding: '24px',
        boxSizing: 'border-box',
      });
      doc.body.appendChild(overlay);
    }
    overlay.replaceChildren();
    const title = doc.createElement('div');
    title.textContent = 'Termian';
    Object.assign(title.style, { fontSize: '22px', fontWeight: '600' });
    const status = doc.createElement('div');
    status.textContent = failed
      ? 'Интерфейс не загрузился. Данные сохранены.'
      : 'Подготовка приложения…';
    Object.assign(status.style, { fontSize: '14px', color: '#afb2c4' });
    overlay.append(title, status);
    if (failed) {
      const retry = doc.createElement('button');
      retry.textContent = 'Повторить загрузку';
      Object.assign(retry.style, {
        padding: '10px 18px', border: '1px solid #70758d',
        borderRadius: '8px', color: '#fff', background: '#292e48',
        font: 'inherit', cursor: 'pointer',
      });
      retry.addEventListener('click', () => {
        safelyRemember(RELOAD_KEY, null);
        hostWindow.location.reload();
      });
      overlay.appendChild(retry);
      const details = doc.createElement('pre');
      details.id = 'termian-startup-errors';
      details.textContent = startupErrors.length
        ? startupErrors.join('\n\n')
        : 'Ошибок JavaScript не зафиксировано: запуск завис без исключения.';
      Object.assign(details.style, {
        maxWidth: 'min(900px, 90vw)', maxHeight: '35vh', overflow: 'auto',
        margin: '0', padding: '12px', borderRadius: '8px', background: '#1c2035',
        color: '#afb2c4', font: '12px/1.5 ui-monospace, monospace',
        textAlign: 'left', whiteSpace: 'pre-wrap', userSelect: 'text',
      });
      overlay.appendChild(details);
    }
  }

  function check() {
    if (!finished && hasRenderedUi()) finish();
  }

  function recover() {
    if (finished) return;
    if (hasRenderedUi()) return finish();
    if (profileIsReady()) {
      // Do not restart the app or stop watching it during a slow warm launch.
      // A delayed React mount should still hide the error automatically.
      if (recoveryTimer !== null) hostWindow.clearTimeout(recoveryTimer);
      recoveryTimer = null;
      console.error('[Termian] Existing profile UI did not render; leaving data untouched');
      displayOverlay(true);
      return;
    }
    clearTimers();
    if (!hasReloaded()) {
      safelyRemember(RELOAD_KEY, '1');
      console.warn('[Termian] Main WebView did not render on first launch; reloading it once');
      // Only the main page reloads: the vault owner, profile and SSH data remain.
      try { hostWindow.location.reload(); }
      catch (error) {
        console.error('[Termian] Main WebView reload failed', error);
        displayOverlay(true);
      }
      return;
    }
    console.error('[Termian] Main WebView is still blank after one recovery attempt');
    displayOverlay(true);
  }

  function begin() {
    if (finished) return;
    root = doc.getElementById('root');
    if (!root) {
      console.error('[Termian] Main WebView has no root element');
      return;
    }
    check();
    if (finished) return;
    // React appends its first top-level child when the initial UI mounts.
    // React may render just after a timer tick: respond to that single mount
    // immediately while keeping the periodic fallback at exactly 2 seconds.
    // Do not observe the entire React subtree (thousands of DOM mutations).
    observer = new hostWindow.MutationObserver(check);
    observer.observe(root, {childList:true});
    interval = hostWindow.setInterval(check, intervalMs);
    showTimer = hostWindow.setTimeout(() => displayOverlay(), overlayDelayMs);
    recoveryTimer = hostWindow.setTimeout(recover, timeoutMs);
  }

  hostWindow.addEventListener('error', event => {
    const where = event?.filename ? ` (${event.filename}:${event.lineno}:${event.colno})` : '';
    rememberError([(event?.message || 'Unknown error') + where, event?.error?.stack].filter(Boolean).join('\n'));
  });
  hostWindow.addEventListener('unhandledrejection', event => {
    rememberError('Unhandled rejection: ' + describeReason(event?.reason));
  });

  if (doc.readyState === 'loading') {
    doc.addEventListener('DOMContentLoaded', begin, { once: true });
  } else {
    begin();
  }
  hostWindow.addEventListener('pagehide', clearTimers, { once: true });
  return { check, recover };
}
