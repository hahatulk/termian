'use strict';

// Bootstrap for the recovered Termius 9.35.4 renderers.
// Their HTML, React bundles, styles and visual assets are reused as-is.
// This file adapts only the old Electron preload/module boundary to Tauri.
//
// Electron ran three renderers which are reproduced as Tauri windows:
//   ui           index.html     main window (connections, terminal, vault UI)
//   preferences  settings.html  the separate Settings window
//   background   background.html hidden data process (IndexedDB vault, known
//                               hosts, persistence, self-cloud sync adapter)
// Their Redux stores are kept in sync by ./bridge/redux-relay.js.
import { Buffer } from 'buffer';
import path from 'path-browserify';
import { EventEmitter } from 'events';
import * as nodeStream from 'readable-stream';
import localAccount from './local-account.cjs';
import selfCloudAdapter from './self-cloud-adapter.cjs';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { buildApplicationMenu, buildMacApplicationMenu } from './bridge/app-menu.js';
import { LogicalPosition } from '@tauri-apps/api/dpi';
import { invoke } from '@tauri-apps/api/core';
import { readText, writeText } from '@tauri-apps/plugin-clipboard-manager';
import { listen } from '@tauri-apps/api/event';
import { createReduxRelay } from './bridge/redux-relay.js';
import { createIpcBus } from './bridge/ipc-bus.js';
import { crypto as libtermiusCrypto } from './bridge/ssh-compat-crypto.js';
import { createLibtermius } from './bridge/ssh-compat.js';
import { createSerialPort } from './bridge/serialport.js';
import { installMainProcess } from './bridge/main-process.js';
import { installStartupRecovery } from './bridge/startup-recovery.js';
import { installSelfCloud, isCredentialError } from './bridge/self-cloud.js';
import { createFsPromises, createKeytar, fsConstants } from './bridge/node-shims.js';
import { createLocalFs } from './bridge/local-fs.js';
import { createFileAccessGuide } from './bridge/file-access-guide.js';
import { installAppRegionTracker } from './bridge/app-region.js';
import { installUpdateNotice, createUpdatesApi } from './bridge/update-notice.js';
import { installSessionLogSync } from './bridge/session-log-sync.js';

(function bootstrapOriginalRenderer() {
  const appVersion = __TERMIAN_APP_VERSION__;
  const mac = /Mac/i.test(navigator.platform || '');
  const platform = mac ? 'darwin' : 'win32';
  const osPlatform = mac ? 'mac' : 'win';
  const params = new URLSearchParams(location.search);
  const processType = params.get('type') || 'ui';
  const isBackground = processType === 'background';
  const isPrimary = processType === 'ui';
  // Install before invoking the native window APIs: even if the initial Tauri
  // bridge fails during a brand-new WebView profile startup, the primary page
  // gets a visible recovery path instead of a permanently black window.
  if (isPrimary) installStartupRecovery();
  const appWindow = getCurrentWindow();

  // Electron's -webkit-app-region: drag CSS does not itself move a frameless
  // Tauri window. Honor the original draggable/no-drag regions using Tauri's
  // native window API, without changing any recovered CSS or blocking buttons.
  // Installed before the UI bundles so WebKit's dropped rules are recorded.
  const appRegions = isBackground ? null : installAppRegionTracker();
  const isOriginalDragRegion = target => !!appRegions?.isDragRegion(target);
  // macOS: native traffic lights replace the UI's decorative placeholder
  // circles (Electron drew real buttons over them); keep their layout space.
  if (mac && !isBackground) {
    const style = document.createElement('style');
    style.textContent = '[class*="-systemBarMainWindow"] > *, [class*="-systemBarSecondaryWindow"] > * { visibility: hidden !important; }';
    (document.head ?? document.documentElement).appendChild(style);
  }
  if (!isBackground) {
    document.addEventListener('mousedown', event => {
      if (event.button !== 0 || event.detail > 1 || !isOriginalDragRegion(event.target)) return;
      void appWindow.startDragging().catch(error => console.error('[Termian] Window drag failed:',error));
    }, true);
    document.addEventListener('dblclick', event => {
      if (event.button !== 0 || !isOriginalDragRegion(event.target) || !isPrimary) return;
      void appWindow.toggleMaximize().then(async () => {
        maximized = await appWindow.isMaximized();
        notifyWindow(maximized ? 'maximize' : 'unmaximize');
      }).catch(error => console.error('[Termian] Window maximize failed:',error));
    }, true);
  }

  // Exact original HTML expects synchronous Electron runtime properties.
  // Only static, nonsensitive environment metadata is supplied here.
  const environment = {
    hostName: 'localhost',
    exePath: mac ? '/Applications/termian.app' : 'termian.exe',
    appPath: 'app',
    homePath: mac ? '/Users/local' : 'C:/Users/local',
    downloadsPath: mac ? '/Users/local/Downloads' : 'C:/Users/local/Downloads',
    appVersion,
    userDataPath: mac ? '/Users/local/Library/Application Support/app.termian.desktop' : 'termian-data',
    locale: navigator.language || 'en-US',
    isUnderArm64Translation: false,
    osVersion: mac ? '15.0' : '10.0',
    osArch: mac ? 'arm64' : 'x64',
    osFamily: mac ? 'macos' : 'windows',
    osUsername: 'local',
    osUserId: 0,
    osInfo: mac ? 'macOS' : 'Windows',
    isDev: false,
    isPackaged: true,
    platform: osPlatform,
    installSource: 'Website',
    ...(mac
      ? {isMacosSandbox:false, shellPath:'/bin/zsh'}
      : {system32Path:'C:/Windows/System32', powershellPath:'C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe', cmdPath:'C:/Windows/System32/cmd.exe'}),
  };

  window.Buffer = Buffer;
  window.__dirname = '/ui-process/src/ui-process';
  window.process = {
    platform,
    type: 'renderer',
    argv: ['--termius-environment=local'],
    env: {NODE_ENV:'production'},
    versions: {electron:'21.0.0',node:'18.0.0'},
    pid: 1,
    // readable-stream (the terminal's output flow control) schedules with it.
    nextTick: (callback, ...args) => queueMicrotask(() => callback(...args)),
  };
  // The SFTP "Local" panel starts in process.env.HOME (ui-process `_he`).
  if (!isBackground) {
    void invoke('local_fs', {op:'home', path:''})
      .then(home => { window.process.env.HOME = home; })
      .catch(error => console.error('[Termian] Home directory lookup failed:', error));
  }

  // The recovered bundles contain CommonJS requires of the self-cloud modules;
  // no general-purpose Node.js loader can safely exist in a Tauri webview.
  window.require = (name) => {
    if (name === 'path') return path;
    const normalized = typeof name === 'string' ? name.replaceAll('\\','/') : '';
    if (normalized.endsWith('/self-cloud/local-account.cjs')) return localAccount;
    if (normalized.endsWith('/self-cloud/adapter.cjs')) return selfCloudAdapter;
    throw new Error('Node require is not available in Tauri: ' + String(name));
  };

  // An inert import placeholder is needed because the original renderer
  // eagerly destructures Electron native modules. Accessing them is harmless;
  // calling a protected operation fails loudly instead of pretending success.
  const unsupported = new Map();
  const protectedOperations = [
    'fs.', 'child_process.', 'net.',
    '@termius/libtermius.TelnetClient', '@termius/libtermius.srp',
  ];
  const mustReject = name => protectedOperations.some(prefix => name.startsWith(prefix));
  const inertModule = (moduleName) => {
    if (unsupported.has(moduleName)) return unsupported.get(moduleName);
    const f = function unsupportedElectronApi() {
      if (mustReject(moduleName)) throw new Error('Unsupported native Electron operation: ' + moduleName);
      return inertModule(moduleName + '()');
    };
    const stub = new Proxy(f, {
      get(_target, property) {
        if (property === 'then') return undefined;
        if (property === Symbol.toPrimitive) return () => moduleName;
        if (property === 'toString') return () => '[Unsupported Electron module: ' + moduleName + ']';
        return inertModule(moduleName + '.' + String(property));
      },
      apply() {
        if (mustReject(moduleName)) throw new Error('Unsupported native Electron operation: ' + moduleName);
        return inertModule(moduleName + '()');
      },
      construct() {
        if (mustReject(moduleName)) throw new Error('Unsupported native Electron operation: ' + moduleName);
        return inertModule(moduleName + ' (constructed)');
      },
    });
    unsupported.set(moduleName, stub);
    return stub;
  };
  // Fill a real implementation object with inert placeholders for anything
  // it does not provide, so destructuring the original exports never fails.
  const withFallback = (moduleName, implementation) => new Proxy(implementation, {
    get(target, property, receiver) {
      if (property in target) return Reflect.get(target, property, receiver);
      if (property === 'then' || typeof property === 'symbol') return undefined;
      return inertModule(moduleName + '.' + property);
    },
  });

  const windowCallbacks = new Map();
  let maximized = false;
  let fullScreen = false;
  const notifyWindow = (event) => {
    for (const handler of windowCallbacks.get(event) ?? []) {
      try { handler(); } catch (error) { console.error(error); }
    }
  };
  const windowId = Number.parseInt(params.get('id') ?? (isBackground ? '1' : '3'), 10);
  const remoteWindow = {
    id: windowId,
    webContents: {id: windowId, send: () => {}},
    isFullScreen: () => fullScreen,
    isMaximized: () => maximized,
    isDestroyed: () => false,
    isFocused: () => document.hasFocus(),
    on(event, cb) {
      if (!windowCallbacks.has(event)) windowCallbacks.set(event, new Set());
      windowCallbacks.get(event).add(cb);
      return remoteWindow;
    },
    addListener(event, cb) { return remoteWindow.on(event, cb); },
    removeListener(event, cb) { windowCallbacks.get(event)?.delete(cb); return remoteWindow; },
    minimize() { return appWindow.minimize(); },
    maximize() {
      return appWindow.maximize().then(() => {
        maximized = true; notifyWindow('maximize');
      });
    },
    unmaximize() {
      return appWindow.unmaximize().then(() => {
        maximized = false; notifyWindow('unmaximize');
      });
    },
    // The Settings window is hidden, not destroyed, exactly like Electron did.
    close() { return isPrimary ? appWindow.close() : mainProcess.hideSettingsWindow(appWindow); },
    hide() { return appWindow.hide(); },
    show() { return appWindow.show(); },
    focus() { return appWindow.setFocus(); },
    setFullScreen(value) {
      return appWindow.setFullscreen(!!value).then(() => {
        fullScreen = !!value;
        notifyWindow(fullScreen ? 'enter-full-screen' : 'leave-full-screen');
      });
    },
    getBounds: () => ({width:window.innerWidth,height:window.innerHeight}),
  };
  void appWindow.onResized(async () => {
    const next = await appWindow.isMaximized().catch(() => maximized);
    if (next !== maximized) {
      maximized = next;
      notifyWindow(maximized ? 'maximize' : 'unmaximize');
    }
  }).catch(() => {});
  void appWindow.onFocusChanged(({payload}) => notifyWindow(payload ? 'focus' : 'blur')).catch(() => {});

  // The original frameless titlebar opens Electron's native application menu.
  // It is rebuilt on every open: labels and enabled items follow the state.
  async function showNativeSystemMenu(position = {}) {
    const store = window.__termianStore;
    if (!store) return;
    const menu = await buildApplicationMenu({
      store, windowId, appWindow, openExternal,
      quit: () => invoke('quit_app'),
    });
    const x = Number.isFinite(position?.x) ? position.x : Number.isFinite(position?.clientX) ? position.clientX : 0;
    const y = Number.isFinite(position?.y) ? position.y : Number.isFinite(position?.clientY) ? position.clientY : 0;
    await menu.popup(new LogicalPosition(x,y),appWindow);
  }

  // macOS: the menu bar replaces the hamburger popup (see bridge/app-menu.js).
  // Rebuilt only when an enabled state or label shown in it changes.
  function installMacAppMenu(store) {
    let shown = null;
    let pending = false;
    const signature = () => {
      const state = store.getState();
      const terminal = state.terminal;
      return [(state.entities?.vaults?.length ?? 0) > 1,
        !!terminal?.activeTabId && !!terminal?.tabs?.some?.(tab => tab.id === terminal.activeTabId && tab.terminalId)].join();
    };
    const refresh = () => {
      if (pending || signature() === shown) return;
      pending = true;
      setTimeout(async () => {
        shown = signature(); // A failed build is not retried in a loop.
        try {
          const menu = await buildMacApplicationMenu({store, windowId, appWindow, quit: () => invoke('quit_app'),
            openFileAccess: () => fileAccessGuide?.open()});
          await menu.setAsAppMenu();
        } catch (error) {
          console.error('[Termian] macOS menu failed:', error);
        } finally {
          pending = false;
        }
        if (signature() !== shown) refresh();
      }, 150);
    };
    store.subscribe(refresh);
    refresh();
  }

  const openExternal = url => invoke('open_external', {url:String(url)})
    .catch(error => console.error('[Termian] Opening external link failed:',error));

  // macOS privacy prompts/denials for the SFTP "Local" panel: a live guide.
  const fileAccessGuide = mac && !isBackground ? createFileAccessGuide({invoke, openExternal}) : null;
  let lastAccessGuide = 0;
  const onPermissionDenied = () => {
    if (!fileAccessGuide || Date.now() - lastAccessGuide < 30_000) return;
    lastAccessGuide = Date.now();
    fileAccessGuide.open();
  };

  // ipcRenderer: channels between renderers travel over a BroadcastChannel;
  // channels that the Electron main process answered are handled here.
  const bus = createIpcBus({source:processType + ':' + windowId});
  const ipc = {
    on(channel, handler) { bus.on(channel,handler); return ipc; },
    addListener(channel, handler) { return ipc.on(channel,handler); },
    once(channel, handler) { bus.once(channel,handler); return ipc; },
    removeListener(channel,handler) { bus.off(channel,handler); return ipc; },
    removeAllListeners(channel) { bus.offAll(channel); return ipc; },
    send(channel,...args) {
      switch (channel) {
        case 'open-popup-menu':
          void showNativeSystemMenu(args[0]).catch(error => console.error('[Termian] Native menu failed:',error));
          return;
        case 'open-external':
          void openExternal(args[0]);
          return;
        case 'update-system-menu':
        case 'trigger-update-notification':
        case 'redux-action':
          return;
        default:
          bus.send(channel,...args);
      }
    },
    sendSync(channel) {
      // Electron answered init-redux with the main store state; see
      // bridge/main-process.js for the snapshot that replaces it.
      if (channel === 'init-redux') return mainProcess.readSnapshot();
      return undefined;
    },
    invoke(channel,...args) {
      if (channel === 'wasAppOpenedWithDeeplink') return Promise.resolve(false);
      if (channel === 'dialog.showOpenDialog' || channel === 'dialog.showSaveDialog') {
        return invoke(channel === 'dialog.showOpenDialog' ? 'show_open_dialog' : 'show_save_dialog', {options:args[0] ?? {}});
      }
      if (channel.startsWith('self-cloud:')) return selfCloud.invoke(channel,...args);
      return bus.invoke(channel,...args);
    },
  };

  const electron = {
    ipcRenderer: ipc,
    ipcMain: ipc,
    app: {
      isPackaged:true, getVersion:()=> appVersion, getName:()=> 'termian',
      getPath:(name)=> name === 'exe' ? environment.exePath : environment.userDataPath,
      getAppPath:()=> environment.appPath,
      on:()=>{}, isReady:()=>true, getLocale:()=>environment.locale,
    },
    shell: {openExternal:async url => { await openExternal(url); return true; }},
    clipboard: {
      // Electron read synchronously; the terminal (the only caller) awaits it.
      readText: () => readText().catch(error => { console.error('[Termian] Clipboard read failed:', error); return ''; }),
      writeText(text) { void writeText(String(text)).catch(console.error); },
    },
    nativeTheme: {shouldUseDarkColors:true, on(){}, removeListener(){}},
  };
  const keytar = createKeytar(invoke);
  const remote = withFallback('@electron/remote', {
    getCurrentWindow: () => remoteWindow,
    getCurrentWebContents: () => remoteWindow.webContents,
    app: electron.app,
    require: (name) => name === '@termius/keytar' ? keytar : inertModule('remote.require(' + name + ')'),
    BrowserWindow: inertModule('remote.BrowserWindow'),
    inAppPurchase: inertModule('remote.inAppPurchase'),
  });

  // Main-process duties (Settings window, snapshots for new windows) live in
  // the primary window; every window takes part in the action relay.
  let terminalBridge = null;
  let sessionLogSync = null;
  const mainProcess = installMainProcess({isPrimary, processType, windowId, invoke});
  const relay = createReduxRelay({
    source: processType + ':' + windowId,
    onAction(action) {
      mainProcess.onAction(action);
      terminalBridge?.onAction?.(action);
    },
  });
  const electronRedux = {
    forwardToMain: relay.forwardToMain,
    replayActionRenderer(store) {
      relay.replayActionRenderer(store);
      mainProcess.attachStore(store);
      window.__termianStore = store;
      if (isPrimary) sessionLogSync ??= installSessionLogSync({invoke, getState: () => store.getState(), isCredentialError});
      if (mac && isPrimary) installMacAppMenu(store);
      if (isPrimary) {
        setTimeout(() => {
          try {
            store.dispatch({type:'ui/changeTheme',payload:{theme:'dark',selectedThemeValue:'dark'}});
          } catch (error) {
            console.error('[Termian] Original UI initialization failed', error);
          }
        }, 350);
      }
    },
  };

  const browserZlib = {
    // The compressed Electron argv is only used for static metadata: the
    // original deserializer still checks the exact original Zod schema.
    brotliDecompressSync: () => Buffer.from(JSON.stringify(environment),'utf8'),
  };

  const libtermius = withFallback('@termius/libtermius', createLibtermius({
    crypto: libtermiusCrypto, invoke, listen,
    onSessionLogFinished: () => sessionLogSync?.sessionFinished(),
  }));

  const shims = new Map([
    ['electron',electron],
    ['@electron/remote',remote],
    ['electron-redux',electronRedux],
    ['path',path],
    // require('events') is the EventEmitter class itself in Node.js.
    ['events',Object.assign(EventEmitter,{EventEmitter})],
    ['zlib',browserZlib],
    ['stream',nodeStream],
    ['@termius/libtermius',libtermius],
    ['@termius/electron-ipc',inertModule('@termius/electron-ipc')],
    ['@termius/mosh',inertModule('@termius/mosh')],
    ['@serialport/stream',createSerialPort({invoke, listen})],
    // Real local files only where the SFTP "Local" panel and Settings run.
    ['fs',withFallback('fs', isBackground ? {constants: fsConstants}
      : {constants: fsConstants, ...createLocalFs({invoke, onPermissionDenied})})],
    ['fs/promises',withFallback('fs/promises', createFsPromises({invoke}))],
    ['child_process',inertModule('child_process')],
    ['crypto',inertModule('crypto')],
    ['net',inertModule('net')],
  ]);
  const get = shims.get.bind(shims);
  shims.get = (name) => get(name) || inertModule(String(name));
  Object.defineProperty(window, Symbol.for('preloadedModulesKey'), {
    value:shims,configurable:false,enumerable:false,writable:false
  });

  const selfCloud = installSelfCloud({invoke, bus, isBackground, isPrimary});
  window.api = {
    selfCloud: selfCloud.api,
    fileImports: {parseSecureCrtConfig(){throw Error('SecureCRT import is not available yet');}},
    ...(isBackground ? {} : {updates: createUpdatesApi({invoke, listen})}),
  };
  if (isPrimary) installUpdateNotice({invoke, listen});
  if (isPrimary) {
    import('./legacy-terminal.js').then(({createOriginalTerminalBridge}) => {
      const attach = () => {
        if (!window.__termianStore) return void setTimeout(attach, 100);
        terminalBridge = createOriginalTerminalBridge({
          invokeNative:invoke,listenNative:listen,store:window.__termianStore,
          terminalOutput:libtermius.terminalOutput,
        });
        window.addEventListener('beforeunload',()=>void terminalBridge?.dispose());
      };
      attach();
    });
  }
  window.addEventListener('error',event=>console.error('[Original UI]',event.message));
  window.addEventListener('unhandledrejection',event=>console.error('[Original UI rejection]',event.reason));
})();
