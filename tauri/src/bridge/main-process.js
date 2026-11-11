// Duties of the Electron main process that the recovered renderers rely on.
// Only the primary (ui) window opens windows; the Settings window is created
// on first use and afterwards hidden/shown, as Electron's preferences window was.
import { WebviewWindow } from '@tauri-apps/api/webviewWindow';

// Electron answered `init-redux` synchronously with the main store's state, so
// a reloaded or newly opened window started with the vault already loaded.
// The background (vault owner) and the primary window publish JSON snapshots
// of their state to localStorage, which every window shares synchronously.
const VAULT_SNAPSHOT_KEY = 'termian:init-redux:vault';
const UI_SNAPSHOT_KEY = 'termian:init-redux:ui';
const VAULT_SLICES = ['entities', 'tags', 'user', 'authorized', 'vault', 'vaultsManagement', 'identityAutocomplete'];
const SETTINGS_LABEL = 'preferences';
const SETTINGS_WINDOW_ID = 4;

function writeSnapshot(key, state, slices) {
  try {
    const value = slices ? Object.fromEntries(slices.filter(name => name in state).map(name => [name, state[name]])) : state;
    localStorage.setItem(key, JSON.stringify(value));
  } catch (error) {
    console.error('[Termian] Cannot store the state snapshot for new windows', error);
  }
}

function readJson(key) {
  try { return JSON.parse(localStorage.getItem(key) ?? 'null'); }
  catch { return null; }
}

export function installMainProcess({ isPrimary, processType }) {
  let store = null;
  let settingsWindow = null;
  let publishTimer = null;

  function publishVault() {
    clearTimeout(publishTimer);
    publishTimer = setTimeout(() => writeSnapshot(VAULT_SNAPSHOT_KEY, store.getState(), VAULT_SLICES), 250);
  }

  async function openSettings() {
    if (!store?.getState().authorized) return;
    const existing = settingsWindow ?? await WebviewWindow.getByLabel(SETTINGS_LABEL);
    if (existing) {
      settingsWindow = existing;
      await existing.show();
      await existing.unminimize().catch(() => {});
      await existing.setFocus();
      store.dispatch({type: 'ui/setSettingsOpenState', payload: true});
      return;
    }
    writeSnapshot(UI_SNAPSHOT_KEY, store.getState());
    settingsWindow = new WebviewWindow(SETTINGS_LABEL, {
      url: `settings.html?type=preferences&id=${SETTINGS_WINDOW_ID}`,
      title: 'Settings',
      width: 1100,
      height: 680,
      minWidth: 1100,
      resizable: false,
      maximizable: false,
      minimizable: false,
      // macOS: native buttons over the UI's placeholder ones, as in Electron.
      ...(/Mac/i.test(navigator.platform || '')
        ? {decorations: true, titleBarStyle: 'overlay', hiddenTitle: true}
        : {decorations: false}),
      center: true,
      focus: true,
      backgroundColor: '#131626',
      // Hidden between uses, like Electron's preferences window (throttling off).
      backgroundThrottling: 'disabled',
    });
    settingsWindow.once('tauri://error', event => {
      console.error('[Termian] Settings window failed:', event.payload);
      settingsWindow = null;
    });
    settingsWindow.once('tauri://destroyed', () => { settingsWindow = null; });
    store.dispatch({type: 'ui/setSettingsOpenState', payload: true});
  }

  return {
    attachStore(target) {
      store = target;
      if (processType === 'background') {
        let entities = null;
        store.subscribe(() => {
          const next = store.getState().entities;
          if (next !== entities) {
            entities = next;
            publishVault();
          }
        });
      }
    },
    // Electron hid the preferences window and reset its navigation on close.
    hideSettingsWindow(appWindow) {
      store?.dispatch({type: 'ui/setSettingsOpenState', payload: false});
      store?.dispatch({type: 'NAVIGATE_SETTINGS', payload: undefined});
      return appWindow.hide();
    },
    readSnapshot() {
      if (processType === 'background') return undefined;
      const vault = readJson(VAULT_SNAPSHOT_KEY) ?? {};
      // The Settings window also mirrors the primary window's UI state.
      const ui = isPrimary ? {} : readJson(UI_SNAPSHOT_KEY) ?? {};
      const state = {...ui, ...vault};
      return Object.keys(state).length ? state : undefined;
    },
    onAction(action) {
      // The background refreshes Logs (entities.histories) after a successful
      // Termius cloud history sync, which never happens in this offline build;
      // otherwise a new connection only shows up once its tab is closed.
      if (processType === 'background' && action.type === 'PUSH_HISTORY_SUCCESS') {
        queueMicrotask(() => store?.dispatch({
          type: 'UPDATE_HISTORY_ENTITIES_REQUEST',
          meta: {requestId: crypto.randomUUID()},
        }));
      }
      if (!isPrimary) return;
      if (action.type === 'OPEN_SETTINGS') {
        void openSettings().catch(error => console.error('[Termian] Opening Settings failed:', error));
      }
    },
  };
}
