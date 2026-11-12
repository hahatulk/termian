// macOS file access guide for the SFTP "Local" panel. Lists the folders the
// app may need and refreshes their status every two seconds while open, so
// the user sees a change as soon as access is granted in System Settings.
// Opening the guide probes the folders, which lets macOS show its one-time
// privacy prompt; a denial can only be changed in System Settings.
const REFRESH_MS = 2000;
const STATUS = {
  granted: ['Доступ есть', 'ok'],
  denied: ['Нет доступа', 'denied'],
  missing: ['Папки нет', 'idle'],
  unknown: ['Не удалось проверить', 'idle'],
};
const STYLE = `
#termian-file-access{position:fixed;inset:0;z-index:2147483646;display:flex;align-items:center;justify-content:center;padding:24px;background:rgba(0,0,0,.54);color:var(--text-primary,#f2f3f5);font:13px/1.45 system-ui,sans-serif;-webkit-app-region:no-drag}
#termian-file-access .fa-dialog{width:560px;max-width:100%;max-height:calc(100vh - 48px);overflow:auto;border:1px solid #454950;border-radius:12px;background:#272b30;box-shadow:0 18px 64px #0006;padding:22px 24px}
#termian-file-access h2{margin:0 0 6px;font-size:17px;font-weight:600}
#termian-file-access p{margin:0 0 14px;color:#9ea5b3;font-size:12px}
#termian-file-access ul{list-style:none;margin:0 0 14px;padding:0;display:flex;flex-direction:column;gap:8px}
#termian-file-access li{display:grid;grid-template-columns:10px minmax(0,1fr) auto;gap:10px;align-items:center;padding:10px 12px;border:1px solid #454950;border-radius:8px;background:#202328}
#termian-file-access .fa-dot{width:8px;height:8px;border-radius:50%;background:#9ea5b3}
#termian-file-access li[data-state=ok] .fa-dot{background:#3dcc94}
#termian-file-access li[data-state=denied] .fa-dot{background:#ec706f}
#termian-file-access .fa-name{font-weight:500}
#termian-file-access .fa-path{display:block;font-size:11px;color:#9ea5b3;overflow-wrap:anywhere}
#termian-file-access .fa-state{font-size:12px;color:#9ea5b3;white-space:nowrap}
#termian-file-access li[data-state=denied] .fa-state{color:#ec706f}
#termian-file-access li[data-state=ok] .fa-state{color:#3dcc94}
#termian-file-access button{min-height:30px;padding:5px 12px;border:1px solid #454950;border-radius:6px;background:#35393f;color:#f2f3f5;font:inherit;font-size:12px;cursor:pointer}
#termian-file-access button:hover{background:#454950}
#termian-file-access .fa-footer{display:flex;justify-content:space-between;align-items:center;gap:12px}
#termian-file-access .fa-updated{font-size:11px;color:#9ea5b3}
`;

export function createFileAccessGuide({ invoke, openExternal }) {
  let modal = null;
  let timer = null;

  function close() {
    clearTimeout(timer);
    timer = null;
    modal?.remove();
    modal = null;
  }

  function render(items) {
    if (!modal) return;
    const list = modal.querySelector('ul');
    list.replaceChildren(...items.map(item => {
      const [label, state] = STATUS[item.status] || STATUS.unknown;
      const row = document.createElement('li');
      row.dataset.state = state;
      const dot = document.createElement('span');
      dot.className = 'fa-dot';
      const text = document.createElement('div');
      const name = document.createElement('span');
      name.className = 'fa-name';
      name.textContent = item.title + (item.required ? '' : ' (необязательно)');
      const path = document.createElement('span');
      path.className = 'fa-path';
      path.textContent = item.path;
      text.append(name, path);
      const side = document.createElement('div');
      if (item.status === 'denied') {
        const open = document.createElement('button');
        open.type = 'button';
        open.textContent = 'Открыть настройки';
        open.onclick = () => void openExternal(item.settingsUrl);
        side.append(open);
      } else {
        const status = document.createElement('span');
        status.className = 'fa-state';
        status.textContent = label;
        side.append(status);
      }
      row.append(dot, text, side);
      return row;
    }));
    modal.querySelector('.fa-updated').textContent = 'Обновлено: ' + new Date().toLocaleTimeString();
  }

  async function refresh() {
    timer = null;
    try { render(await invoke('local_fs_permissions')); }
    catch (error) { console.error('[Termian] File access check failed', error); }
    if (modal) timer = setTimeout(refresh, REFRESH_MS);
  }

  function open() {
    if (modal) return;
    if (!document.getElementById('termian-file-access-style')) {
      const style = document.createElement('style');
      style.id = 'termian-file-access-style';
      style.textContent = STYLE;
      document.head.appendChild(style);
    }
    modal = document.createElement('div');
    modal.id = 'termian-file-access';
    modal.innerHTML = `<div class="fa-dialog" role="dialog" aria-modal="true" aria-labelledby="fa-title">
      <h2 id="fa-title">Доступ к файлам</h2>
      <p>Панель Local в SFTP работает с файлами этого Mac. macOS один раз спрашивает разрешение на Рабочий стол, Документы и Загрузки. Если доступ был отклонён, включите termian в Системные настройки → Конфиденциальность и безопасность → Файлы и папки. Статус ниже обновляется автоматически.</p>
      <ul aria-live="polite"></ul>
      <div class="fa-footer"><span class="fa-updated"></span><button type="button" data-close>Закрыть</button></div>
    </div>`;
    modal.querySelector('[data-close]').onclick = close;
    modal.addEventListener('click', event => { if (event.target === modal) close(); });
    modal.addEventListener('keydown', event => { if (event.key === 'Escape') close(); });
    document.body.appendChild(modal);
    modal.querySelector('[data-close]').focus();
    void refresh();
  }

  return { open, close, isOpen: () => !!modal };
}
