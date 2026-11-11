'use strict';
const styles = require('./self-cloud.css');
const { groupSyncChanges, countLabel } = require('./sync-change-groups.cjs');
const THEME_VARIABLES = ['--foreground', '--background', '--card-a', '--card-b', '--card-c', '--text-primary', '--text-common', '--text-secondary', '--text-error', '--text-success', '--accent', '--accent-dark', '--accent-light', '--border-color', '--input-border-focused', '--input-color', '--disabled-button-bg'];

function installStyles() {
  if (document.getElementById('self-cloud-styles')) return;
  // Tauri's CSP allows inline styles but not file: URLs; the stylesheet is
  // bundled as text by scripts/stage-original-ui.mjs.
  const style = document.createElement('style');
  style.id = 'self-cloud-styles';
  style.textContent = styles;
  document.head.appendChild(style);
}

function createDialog(title, content, actions) {
  installStyles();
  const previouslyFocused = document.activeElement;
  const modal = document.createElement('div');
  modal.id = 'self-cloud-modal'; modal.className = 'self-cloud-overlay';
  modal.innerHTML = `<form class="self-cloud-dialog" role="dialog" aria-modal="true" aria-labelledby="self-cloud-title" autocomplete="off">
    <header class="self-cloud-dialog-header"><div class="self-cloud-heading"><span class="self-cloud-symbol" aria-hidden="true"><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6"><path d="M7 18H6a4 4 0 0 1-.7-7.94 6.5 6.5 0 0 1 12.44-1.42A4.7 4.7 0 0 1 18 18h-1"/><path d="M12 21V11m-3 3 3-3 3 3"/></svg></span><div><h2 id="self-cloud-title"></h2></div></div><button type="button" class="self-cloud-icon-button" data-close aria-label="Закрыть" title="Закрыть"><svg viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.6" aria-hidden="true"><path d="m5 5 10 10M15 5 5 15"/></svg></button></header>
    <div class="self-cloud-fields">${content}<output role="status" aria-live="polite" class="self-cloud-status"></output></div>
    <footer class="self-cloud-actions"><button type="button" class="self-cloud-button self-cloud-cancel" data-close>Закрыть</button>${actions}</footer>
  </form>`;
  modal.querySelector('h2').textContent = title;
  const applyTheme = () => {
    const source = document.querySelector('.termius-dark-theme') || document.querySelector('#root') || document.body;
    const theme = window.getComputedStyle(source);
    for (const name of THEME_VARIABLES) {
      const value = theme.getPropertyValue(name).trim();
      if (value) modal.style.setProperty(name, value);
    }
  };
  applyTheme(); document.body.appendChild(modal);
  const themeObserver = new MutationObserver(applyTheme);
  themeObserver.observe(document.documentElement, { attributes: true, subtree: true, attributeFilter: ['class', 'data-theme'] });
  const form = modal.querySelector('form'), out = modal.querySelector('output');
  let busy = false;
  const close = () => {
    if (busy) return;
    themeObserver.disconnect();
    for (const input of modal.querySelectorAll('input[type=password]')) input.value = '';
    modal.remove();
    if (previouslyFocused?.isConnected) previouslyFocused.focus();
  };
  const setBusy = value => {
    busy = value;
    for (const button of modal.querySelectorAll('button')) button.disabled = value;
    for (const input of modal.querySelectorAll('input')) input.readOnly = value;
  };
  form.addEventListener('submit', event => event.preventDefault());
  modal.addEventListener('keydown', event => {
    event.stopPropagation();
    if (event.key === 'Escape') { event.preventDefault(); close(); }
    if (event.key === 'Tab') {
      const elements = Array.from(modal.querySelectorAll('input,button')).filter(el => !el.disabled && !el.hidden);
      const first = elements[0], last = elements[elements.length - 1];
      if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last?.focus(); }
      if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    }
  });
  modal.addEventListener('click', event => { if (event.target === modal) close(); });
  for (const button of modal.querySelectorAll('[data-close]')) button.onclick = close;
  return { modal, form, out, close, setBusy };
}

async function editConfig(ipcRenderer) {
  const view = createDialog('Своё облако',
    '<label>Адрес сервера<input name="url" type="url" required spellcheck="false" placeholder="https://cloud.example/v1/snapshot" autocomplete="off"></label><label>Токен доступа<input name="token" type="password" autocomplete="new-password" placeholder="Введите токен сервера"><span data-token-state class="self-cloud-secret-state"></span></label><label>Пароль шифрования<input name="passphrase" type="password" minlength="12" autocomplete="new-password" placeholder="Не менее 12 символов"><span data-passphrase-state class="self-cloud-secret-state"></span></label><fieldset class="self-cloud-basic"><legend>HTTP Basic auth</legend><label class="self-cloud-check"><input name="basicEnabled" type="checkbox"> Сервер за прокси с Basic auth</label><label>Логин<input name="basicUser" type="text" spellcheck="false" autocomplete="off"></label><label>Пароль<input name="basicPassword" type="password" autocomplete="new-password"><span data-basic-state class="self-cloud-secret-state"></span></label></fieldset><p class="self-cloud-help">Токен и пароли сохраняются в конфиге этого профиля. Сохранённые значения не передаются в интерфейс. Чтобы заменить секрет, введите новый; пустое поле оставляет прежний.</p>',
    '<button type="button" class="self-cloud-button self-cloud-primary" data-save-config>Сохранить настройки</button>');
  const { modal, form, out, setBusy } = view;
  const applyMetadata = metadata => {
    form.elements.token.required = !metadata.hasToken;
    form.elements.passphrase.required = !metadata.hasPassphrase;
    modal.querySelector('[data-token-state]').textContent = metadata.hasToken ? 'Токен сохранён. Поле — только для замены.' : 'Токен ещё не сохранён.';
    modal.querySelector('[data-passphrase-state]').textContent = metadata.hasPassphrase ? 'Пароль сохранён. Поле — только для замены.' : 'Пароль ещё не сохранён.';
    form.elements.basicEnabled.checked = !!metadata.basicUser;
    form.elements.basicUser.value = metadata.basicUser || '';
    form.elements.basicPassword.dataset.saved = metadata.hasBasicPassword ? '1' : '';
    modal.querySelector('[data-basic-state]').textContent = metadata.hasBasicPassword ? 'Пароль сохранён. Поле — только для замены.' : '';
    syncBasic();
  };
  const syncBasic = () => {
    const on = form.elements.basicEnabled.checked;
    for (const name of ['basicUser', 'basicPassword']) form.elements[name].disabled = !on;
    form.elements.basicUser.required = on;
    form.elements.basicPassword.required = on && !form.elements.basicPassword.dataset.saved;
  };
  form.elements.basicEnabled.onchange = syncBasic;
  setBusy(true);
  try {
    const metadata = await ipcRenderer.invoke('self-cloud:get-config');
    if (metadata?.url) form.elements.url.value = metadata.url;
    applyMetadata(metadata || {});
  } catch (_) { applyMetadata({}); out.textContent = 'Не удалось прочитать настройки.'; out.dataset.state = 'error'; }
  finally { setBusy(false); }
  modal.querySelector('[data-save-config]').onclick = async () => {
    if (!form.reportValidity()) return;
    const changes = { url: form.elements.url.value };
    // Write-only edits: never ask the main process to return a saved secret.
    if (form.elements.token.value) changes.token = form.elements.token.value;
    if (form.elements.passphrase.value) changes.passphrase = form.elements.passphrase.value;
    changes.basicUser = form.elements.basicEnabled.checked ? form.elements.basicUser.value.trim() : '';
    if (changes.basicUser && form.elements.basicPassword.value) changes.basicPassword = form.elements.basicPassword.value;
    setBusy(true); out.textContent = 'Сохранение…'; out.dataset.state = 'working';
    try {
      const result = await ipcRenderer.invoke('self-cloud:save-config', changes);
      if (!result?.ok) throw new Error(result?.error || 'Не удалось сохранить настройки');
      form.elements.token.value = ''; form.elements.passphrase.value = ''; form.elements.basicPassword.value = '';
      applyMetadata(result);
      out.dataset.state = 'success'; out.textContent = 'Настройки сохранены';
    } catch (error) { out.dataset.state = 'error'; out.textContent = error?.message || 'Не удалось сохранить настройки'; }
    finally { setBusy(false); }
  };
  form.elements.url.focus();
  return modal;
}

function displaySyncChanges(modal, status) {
  const panel = modal.querySelector('[data-sync-preview]');
  if (!panel) return;
  panel.hidden = false;
  panel.replaceChildren();
  const headline = document.createElement('strong');
  const summaries = {
    checking: 'Проверяю изменения…',
    syncing: 'Синхронизация…',
    waiting: 'Ожидание локального хранилища…',
    unconfigured: 'Облако не настроено',
    synced: 'Данные клиента и сервера совпадают',
    empty: 'Снимок в облаке пуст',
    upload: 'Есть новые локальные изменения → требуется отправка в облако',
    download: 'В облаке появились новые данные → требуется получение на клиент',
    conflict: 'На обеих сторонах есть изменения — возможен конфликт',
    uncertain: 'Снимки отличаются, но направление изменений пока неизвестно',
    error: 'Не удалось проверить состояние синхронизации',
  };
  headline.textContent = summaries[status?.kind] || 'Статус неизвестен';
  panel.dataset.state = status?.kind || 'unknown';
  panel.appendChild(headline);
  if (status?.kind === 'uncertain') {
    const warning = document.createElement('p');
    warning.textContent = 'Нет подтверждённой общей версии. «Синхронизировать» объединит записи, совпадающие по названию, и предложит выбрать версию, если они различаются; отправка или загрузка полностью заменит одну из сторон.';
    panel.appendChild(warning);
  }
  if (status?.kind === 'conflict') {
    const warning = document.createElement('p');
    warning.textContent = 'Изменения разных записей объединяются автоматически. Если одна и та же запись изменена на обеих сторонах, нужно выбрать, чью версию оставить.';
    panel.appendChild(warning);
  }
  if (status?.kind === 'error' && status.error) {
    const error = document.createElement('p');
    error.textContent = status.error;
    panel.appendChild(error);
    if (status.credentials) {
      const hint = document.createElement('p');
      hint.textContent = 'Автоматическая проверка приостановлена, чтобы не продлевать блокировку. Исправьте токен или пароль в настройках своего облака.';
      panel.appendChild(hint);
    }
  }
  if (Array.isArray(status?.changes) && status.changes.length) {
    const groups = groupSyncChanges(status.changes);
    const overview = document.createElement('div');
    overview.className = 'self-cloud-change-overview';
    overview.textContent = countLabel(status.total || status.changes.length) +
      ' · ' + groups.length + ' ' + (groups.length === 1 ? 'группа' : groups.length < 5 ? 'группы' : 'групп');
    panel.appendChild(overview);
    const list = document.createElement('div');
    list.className = 'self-cloud-change-groups';
    for (const group of groups) {
      const card = document.createElement('details');
      card.className = 'self-cloud-change-group';
      card.dataset.direction = group.direction;
      card.dataset.action = group.action;
      card.open = group.total <= 8;
      const summary = document.createElement('summary');
      const name = document.createElement('span');
      name.className = 'self-cloud-change-action';
      name.textContent = group.actionLabel + ' · ' + group.countLabel;
      const where = document.createElement('span');
      where.className = 'self-cloud-change-location';
      where.textContent = group.directionLabel;
      const caret = document.createElement('span');
      caret.className = 'self-cloud-change-chevron';
      caret.setAttribute('aria-hidden', 'true');
      caret.textContent = '⌄';
      summary.append(name, where, caret);
      card.appendChild(summary);
      const contents = document.createElement('div');
      contents.className = 'self-cloud-change-contents';
      for (const category of group.categories) {
        const row = document.createElement('div');
        row.className = 'self-cloud-change-category';
        const title = document.createElement('span');
        title.className = 'self-cloud-change-category-title';
        title.textContent = category.label + (category.total > 1 ? ' ×' + category.total : '');
        row.appendChild(title);
        if (category.kind === 'other' && category.entries.every(item => !item.hasLabel)) {
          const message = document.createElement('span');
          message.className = 'self-cloud-change-names';
          message.textContent = category.entries.map(item => item.text).join('; ');
          row.appendChild(message);
        } else if (category.entries.some(item => item.detail)) {
          const details = document.createElement('div');
          details.className = 'self-cloud-change-records';
          for (const item of category.entries) {
            const record = document.createElement('div');
            record.className = 'self-cloud-change-record';
            record.textContent = '«' + item.label + '»';
            if (item.detail) {
              const changed = document.createElement('small');
              changed.textContent = item.detail;
              record.appendChild(changed);
            }
            details.appendChild(record);
          }
          row.appendChild(details);
        } else {
          const names = document.createElement('span');
          names.className = 'self-cloud-change-names';
          names.textContent = category.entries.map(item => '«' + item.label + '»').join(', ');
          row.appendChild(names);
        }
        contents.appendChild(row);
      }
      card.appendChild(contents);
      list.appendChild(card);
    }
    panel.appendChild(list);
    if (status.total > status.changes.length) {
      const rest = document.createElement('p');
      rest.textContent = 'И ещё ' + (status.total - status.changes.length) + ' изменений (показаны первые 100).';
      panel.appendChild(rest);
    }
  }
  if (status?.checkedAt) {
    const checked = document.createElement('small');
    checked.textContent = 'Проверено: ' + new Date(status.checkedAt).toLocaleTimeString();
    panel.appendChild(checked);
  }
}

const TITLES = { upload: 'Отправить изменения', download: 'Получить изменения', sync: 'Синхронизация' };

async function synchronize(ipcRenderer, operation) {
  const view = createDialog(TITLES[operation],
    '<section class="self-cloud-preview" data-sync-preview hidden aria-live="polite"></section><div class="self-cloud-notice" data-sync-warning hidden></div>',
    '<div class="self-cloud-sync-actions"><button type="button" class="self-cloud-button self-cloud-primary" data-edit-config hidden>Настроить своё облако</button><button type="button" class="self-cloud-button" data-prefer="remote" hidden>Взять версии из облака</button><button type="button" class="self-cloud-button" data-prefer="local" hidden>Оставить версии устройства</button><button type="button" class="self-cloud-button self-cloud-primary" data-confirm-sync hidden>Подтвердить</button></div>');
  const { modal, out, close, setBusy } = view;
  const configure = modal.querySelector('[data-edit-config]');
  configure.onclick = () => { close(); open(ipcRenderer); };
  try {
    const metadata = await ipcRenderer.invoke('self-cloud:get-config');
    if (!modal.isConnected) return modal;
    if (!metadata?.configured) {
      out.textContent = 'Сначала сохраните адрес сервера, токен и пароль в настройках своего облака.';
      configure.hidden = false;
      return modal;
    }
    out.textContent = 'Проверяю изменения в облаке и на устройстве…';
    out.dataset.state = 'working';
    try {
      const status = await ipcRenderer.invoke('self-cloud:check');
      displaySyncChanges(modal, status);
      out.textContent = '';
    } catch (error) {
      displaySyncChanges(modal, {kind: 'error', error: error?.message || String(error)});
      out.textContent = '';
    }
    // Chromium's blocking confirm() can leave Electron 21 inputs unfocused on
    // Windows. Keep confirmation in this DOM modal, without opening an OS window.
    const warning = modal.querySelector('[data-sync-warning]');
    warning.textContent = operation === 'download' ? 'Данные приложения будут заменены снимком из облака. Перед заменой будет создана зашифрованная резервная копия.'
      : operation === 'sync' ? 'Изменения устройства и облака будут объединены: новые, изменённые и удалённые записи переносятся в обе стороны. Перед применением изменений из облака создаётся зашифрованная резервная копия.'
      : 'Снимок в облаке будет полностью заменён текущими данными. Изменения других устройств будут затёрты.';
    warning.hidden = false;
    const confirm = modal.querySelector('[data-confirm-sync]');
    const preferButtons = [...modal.querySelectorAll('[data-prefer]')];
    confirm.textContent = operation === 'download' ? 'Получить и заменить' : operation === 'sync' ? 'Синхронизировать' : 'Отправить и заменить';
    const run = async request => {
      setBusy(true); confirm.hidden = true;
      for (const button of preferButtons) button.hidden = true;
      out.dataset.state = 'working'; out.textContent = 'Выполняется…';
      try {
        const result = await ipcRenderer.invoke('self-cloud:action', request);
        if (result?.conflicts?.length) {
          displaySyncChanges(modal, {kind: 'conflict', changes: result.conflicts, total: result.conflicts.length});
          out.dataset.state = 'error'; out.textContent = result.error;
          for (const button of preferButtons) button.hidden = false;
          return;
        }
        if (!result?.ok) throw new Error(result?.error || 'Операция не выполнена');
        out.dataset.state = 'success'; out.textContent = result.status || 'Готово';
        displaySyncChanges(modal, await ipcRenderer.invoke('self-cloud:status'));
      } catch (error) {
        out.dataset.state = 'error'; out.textContent = error?.message || 'Операция не выполнена';
        confirm.hidden = false;
      } finally { setBusy(false); }
    };
    confirm.onclick = () => run({ operation });
    for (const button of preferButtons) button.onclick = () => run({ operation: 'sync', prefer: button.dataset.prefer });
    confirm.hidden = false;
    confirm.focus();
  } catch (error) { out.dataset.state = 'error'; out.textContent = error?.message || 'Операция не выполнена'; }
  finally { setBusy(false); }
  return modal;
}

function open(ipcRenderer, preferredOperation) {
  if (typeof document === 'undefined') return Promise.reject(new Error('UI недоступен'));
  const existing = document.getElementById('self-cloud-modal');
  if (existing) { (existing.querySelector('input') || existing.querySelector('button'))?.focus(); return Promise.resolve(existing); }
  return Object.prototype.hasOwnProperty.call(TITLES, preferredOperation)
    ? synchronize(ipcRenderer, preferredOperation) : editConfig(ipcRenderer);
}
module.exports = { open };
