// The application menus that Electron's main process built (main-process `Nje`):
// the Windows titlebar hamburger popup and the macOS menu bar, rebuilt with Tauri menus.
// Items dispatch the same Redux actions the Electron menu handlers did.
import { Menu, MenuItem, PredefinedMenuItem, Submenu } from '@tauri-apps/api/menu';

const separator = () => PredefinedMenuItem.new({item: 'Separator'});

function isVaultsLabel(state) {
  return (state.entities?.vaults?.length ?? 0) > 1;
}

function activeTerminalExists(state) {
  const terminal = state.terminal;
  return !!terminal?.activeTabId && !!terminal?.tabs?.some?.(tab => tab.id === terminal.activeTabId && tab.terminalId);
}

async function sharedSubmenus({ store, windowId, appWindow, selectAllText = false }) {
  const state = store.getState();
  const dispatch = action => store.dispatch(action);
  const terminalOpen = activeTerminalExists(state);
  const item = (text, action, accelerator, options = {}) => MenuItem.new({text, action, accelerator, ...options});
  const selectAll = () => {
    dispatch({type: 'SELECT_ALL_IN_ACTIVE_CONTEXT', payload: {windowId, source: 'SystemMenu'}});
    // A macOS menu key equivalent swallows Cmd+A; Electron also called
    // webContents.selectAll() so inputs keep selecting their text.
    if (selectAllText) document.execCommand('selectAll');
  };

  const edit = await Submenu.new({
    text: 'Edit',
    items: await Promise.all([
      PredefinedMenuItem.new({item: 'Undo'}),
      PredefinedMenuItem.new({item: 'Redo'}),
      separator(),
      PredefinedMenuItem.new({item: 'Cut'}),
      PredefinedMenuItem.new({item: 'Copy'}),
      PredefinedMenuItem.new({item: 'Paste'}),
      item('Select All', selectAll, 'CmdOrCtrl+A'),
      separator(),
      item('Find', () => dispatch({type: 'OPEN_TERMINAL_SEARCH', payload: {windowId, source: 'SystemMenu'}})),
    ]),
  });
  const view = await Submenu.new({
    text: 'View',
    items: await Promise.all([
      item('Toggle Full Screen', async () => appWindow.setFullscreen(!(await appWindow.isFullscreen()))),
      separator(),
      item('Show terminal side-panel', () => dispatch({type: 'sidepanel/toggleSidepanel', payload: {windowId, openedFrom: 'Menu Bar Item'}}), undefined, {enabled: terminalOpen}),
      separator(),
      item('Increase terminal font size', () => dispatch({type: 'INCREASE_TERMINAL_FONT_SIZE', payload: windowId}), 'CmdOrCtrl+=', {enabled: terminalOpen}),
      item('Decrease terminal font size', () => dispatch({type: 'DECREASE_TERMINAL_FONT_SIZE', payload: windowId}), 'CmdOrCtrl+-', {enabled: terminalOpen}),
      item('Reset terminal font size to default', () => dispatch({type: 'RESET_TERMINAL_FONT_SIZE_TO_DEFAULT', payload: windowId}), undefined, {enabled: terminalOpen}),
    ]),
  });
  return { state, dispatch, item, edit, view };
}

export async function buildApplicationMenu({ store, windowId, appWindow, quit }) {
  const { state, dispatch, item, edit, view } = await sharedSubmenus({ store, windowId, appWindow });
  const windowMenu = await Submenu.new({
    text: 'Window',
    items: await Promise.all([PredefinedMenuItem.new({item: 'Minimize'})]),
  });

  return Menu.new({
    items: await Promise.all([
      item(isVaultsLabel(state) ? 'Vaults' : 'Vault', () => dispatch({type: 'OPEN_VAULTS_TAB', payload: {windowId}}), 'Ctrl+1'),
      item('SFTP', () => dispatch({type: 'OPEN_SFTP_TAB', payload: {windowId}}), 'Ctrl+2'),
      item('Settings', () => dispatch({type: 'OPEN_SETTINGS', payload: undefined}), 'CmdOrCtrl+,'),
      separator(),
      item('New Local Terminal', () => dispatch({type: 'OPEN_LOCAL_TERMINAL', payload: {windowId}})),
      item('New Serial Connection', () => dispatch({type: 'MAYBE_OPEN_SERIAL_CONNECTION', payload: {windowId, source: 'System Top Bar'}})),
      edit,
      view,
      windowMenu,
      separator(),
      item('Exit', quit),
    ]),
  });
}

// macOS menu bar, as Electron's main process set it with
// Menu.setApplicationMenu(): the UI shows no hamburger button on macOS, so
// Settings and the clipboard key equivalents are only reachable from here.
export async function buildMacApplicationMenu({ store, windowId, appWindow, quit, openFileAccess }) {
  const { state, dispatch, item, edit, view } = await sharedSubmenus({ store, windowId, appWindow, selectAllText: true });
  const app = await Submenu.new({
    text: 'termian',
    items: await Promise.all([
      PredefinedMenuItem.new({item: {About: null}}),
      separator(),
      item(isVaultsLabel(state) ? 'Vaults' : 'Vault', () => dispatch({type: 'OPEN_VAULTS_TAB', payload: {windowId}})),
      item('SFTP', () => dispatch({type: 'OPEN_SFTP_TAB', payload: {windowId}})),
      item('Settings…', () => dispatch({type: 'OPEN_SETTINGS', payload: undefined}), 'CmdOrCtrl+,'),
      item('Доступ к файлам…', () => openFileAccess?.()),
      separator(),
      PredefinedMenuItem.new({item: 'Services'}),
      separator(),
      PredefinedMenuItem.new({item: 'Hide'}),
      PredefinedMenuItem.new({item: 'HideOthers'}),
      PredefinedMenuItem.new({item: 'ShowAll'}),
      separator(),
      item('Quit termian', quit, 'CmdOrCtrl+Q'),
    ]),
  });
  const file = await Submenu.new({
    text: 'File',
    items: await Promise.all([
      item('New Local Terminal', () => dispatch({type: 'OPEN_LOCAL_TERMINAL', payload: {windowId}})),
      item('New Serial Connection', () => dispatch({type: 'MAYBE_OPEN_SERIAL_CONNECTION', payload: {windowId, source: 'System Top Bar'}})),
    ]),
  });
  const windowMenu = await Submenu.new({
    text: 'Window',
    items: await Promise.all([
      PredefinedMenuItem.new({item: 'Minimize'}),
      PredefinedMenuItem.new({item: 'Maximize'}),
      separator(),
      PredefinedMenuItem.new({item: 'BringAllToFront'}),
    ]),
  });
  return Menu.new({items: [app, file, edit, view, windowMenu]});
}
