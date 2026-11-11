import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { installStartupRecovery } from '../src/bridge/startup-recovery.js';

function fakeFirstLaunch(sharedStorage = new Map(), profileStorage = new Map()) {
  let now = 0;
  let id = 0;
  const timers = new Map();
  const events = new Map();
  const set = (fn, delay, repeat = false) => {
    const key = ++id;
    timers.set(key, { fn, at: now + delay, delay, repeat });
    return key;
  };
  function advance(duration) {
    const end = now + duration;
    for (let guard = 0; guard < 10000; guard++) {
      const pending = [...timers].sort((a, b) => a[1].at - b[1].at)
        .find(([, timer]) => timer.at <= end);
      if (!pending) break;
      const [key, timer] = pending;
      now = timer.at;
      if (timer.repeat) timer.at += timer.delay;
      else timers.delete(key);
      timer.fn();
    }
    now = end;
  }
  class Element {
    constructor() {
      this.style = {};
      this.children = [];
      this.textContent = '';
      this.innerText = '';
      this.firstElementChild = null;
      this.listeners = new Map();
      this.removed = false;
    }
    appendChild(child) { this.children.push(child); return child; }
    append(...children) { children.forEach(child => this.appendChild(child)); }
    replaceChildren(...children) { this.children = children; }
    remove() { this.removed = true; }
    addEventListener(name, callback) { this.listeners.set(name, callback); }
  }
  const root = new Element();
  const body = new Element();
  const observers = [];
  const doc = {
    readyState: 'complete',
    body,
    createElement: () => new Element(),
    getElementById: id => id === 'root' ? root : null,
    addEventListener: (name, callback) => events.set(name, callback),
  };
  let reloads = 0;
  const hostWindow = {
    document: doc,
    sessionStorage: {
      getItem(key) { return sharedStorage.get(key) ?? null; },
      setItem(key, value) { sharedStorage.set(key, value); },
      removeItem(key) { sharedStorage.delete(key); },
    },
    localStorage: {
      getItem(key) { return profileStorage.get(key) ?? null; },
      setItem(key, value) { profileStorage.set(key, value); },
      removeItem(key) { profileStorage.delete(key); },
    },
    MutationObserver: class {
      constructor(callback) { this.callback = callback; this.disconnected = false; observers.push(this); }
      observe(node, options) { this.node = node; this.options = options; }
      disconnect() { this.disconnected = true; }
    },
    setTimeout: (callback, delay) => set(callback, delay),
    clearTimeout: key => timers.delete(key),
    setInterval: (callback, delay) => set(callback, delay, true),
    clearInterval: key => timers.delete(key),
    addEventListener: (name, callback) => events.set(name, callback),
    location: { reload() { reloads++; } },
  };
  return {
    hostWindow, doc, root, body, advance, sharedStorage, profileStorage,
    pollIntervals: () => [...timers.values()].filter(timer => timer.repeat).map(timer => timer.delay),
    notifyRootMounted: () => observers.filter(observer => !observer.disconnected)
      .forEach(observer => observer.callback([{type:'childList',target:root}])),
    observers,
    emit: (name, event) => events.get(name)?.(event),
    reloads: () => reloads, overlay: () => body.children.find(child => child.id === 'termian-startup-status'),
  };
}

test('already rendered UI is never reloaded and does not show a startup overlay', () => {
  const f = fakeFirstLaunch();
  f.root.firstElementChild = {};
  f.root.innerText = 'Hosts';
  installStartupRecovery({ hostWindow: f.hostWindow });
  f.advance(25000);
  assert.equal(f.reloads(), 0);
  assert.equal(f.overlay(), undefined);
});

test('fresh profile shows preparation once and records readiness only after a rendered UI', () => {
  const f = fakeFirstLaunch(new Map([['user-session-marker', 'keep']]));
  installStartupRecovery({ hostWindow: f.hostWindow });
  assert.deepEqual(f.pollIntervals(), [2000], 'Readiness polling must happen once every 2 seconds');
  f.advance(700);
  assert.ok(f.overlay(), 'An empty profile should show initial preparation');
  assert.equal(f.overlay().children[1].textContent, 'Подготовка приложения…');
  assert.equal(f.profileStorage.has('termian:startup:profile-ready'), false,
    'Do not mark unfinished initialization as ready');
  f.root.firstElementChild = {};
  f.root.innerText = 'Hosts and Vault';
  f.advance(1299);
  assert.equal(f.overlay().removed, false, 'Check at 2000ms, not every 250ms');
  f.advance(1);
  assert.equal(f.overlay().removed, true);
  assert.equal(f.profileStorage.get('termian:startup:profile-ready'), '1');
  f.advance(15000);
  assert.equal(f.reloads(), 0);
  assert.equal(f.sharedStorage.get('user-session-marker'), 'keep');
});

test('first React mount ends preparation immediately without waiting for the 2-second health poll', () => {
  const f = fakeFirstLaunch();
  installStartupRecovery({hostWindow:f.hostWindow});
  f.advance(750);
  assert.ok(f.overlay());
  f.root.firstElementChild = {};
  f.root.innerText = 'Hosts';
  f.notifyRootMounted();
  assert.equal(f.overlay().removed,true);
  assert.equal(f.profileStorage.get('termian:startup:profile-ready'),'1');
  assert.deepEqual(f.pollIntervals(),[]);
  assert.equal(f.observers[0].disconnected,true);
});

test('subsequent launches of a ready profile skip preparation even if React renders slowly', () => {
  const profile = new Map([['termian:startup:profile-ready', '1'], ['user-preferences', 'keep']]);
  const f = fakeFirstLaunch(new Map(), profile);
  installStartupRecovery({ hostWindow: f.hostWindow });
  f.advance(1700);
  assert.equal(f.overlay(), undefined, 'Existing profile should not show preparation on normal startup');
  f.root.firstElementChild = {};
  f.root.innerText = 'Hosts';
  f.advance(300);
  assert.equal(f.overlay(), undefined);
  assert.equal(f.reloads(), 0);
  assert.equal(profile.get('user-preferences'), 'keep');
});

test('warm profile with a slow renderer never auto-reloads or shows preparation', () => {
  const profile = new Map([['termian:startup:profile-ready', '1']]);
  const f = fakeFirstLaunch(new Map(),profile);
  installStartupRecovery({hostWindow:f.hostWindow});
  f.advance(10001);
  assert.equal(f.reloads(),0,'Slow warm startup must not reload the entire WebView');
  assert.equal(f.overlay().children[1].textContent,'Интерфейс не загрузился. Данные сохранены.');
  assert.equal(f.sharedStorage.has('termian:startup:reload-attempted'),false);
  // A late React mount must still remove the error without another reload.
  f.root.firstElementChild = {};
  f.root.innerText = 'Hosts';
  f.notifyRootMounted();
  assert.equal(f.overlay().removed,true);
  assert.equal(f.reloads(),0);
});

test('failed startup shows captured JavaScript errors instead of hiding them', () => {
  const profile = new Map([['termian:startup:profile-ready', '1']]);
  const f = fakeFirstLaunch(new Map(), profile);
  installStartupRecovery({hostWindow:f.hostWindow});
  f.emit('error', {message: 'TypeError: x is undefined', filename: 'tauri://localhost/assets/ui.js', lineno: 7, colno: 3});
  f.emit('unhandledrejection', {reason: new Error('vault open failed')});
  f.advance(10001);
  const details = f.overlay().children.find(child => child.id === 'termian-startup-errors');
  assert.ok(details, 'Technical startup errors must stay visible');
  assert.match(details.textContent, /TypeError: x is undefined/);
  assert.match(details.textContent, /ui\.js:7:3/);
  assert.match(details.textContent, /vault open failed/);
});

test('failed startup without JavaScript errors says so explicitly', () => {
  const f = fakeFirstLaunch(new Map(), new Map([['termian:startup:profile-ready', '1']]));
  installStartupRecovery({hostWindow:f.hostWindow});
  f.advance(10001);
  const details = f.overlay().children.find(child => child.id === 'termian-startup-errors');
  assert.match(details.textContent, /не зафиксировано/);
});

test('missing WebView profile after reinstall or update triggers preparation again', () => {
  const existing = new Map([['termian:startup:profile-ready', '1']]);
  const oldLaunch = fakeFirstLaunch(new Map(), existing);
  installStartupRecovery({ hostWindow: oldLaunch.hostWindow });
  oldLaunch.advance(1000);
  assert.equal(oldLaunch.overlay(), undefined);

  // The app-data/WebView directory is absent: it has a new persistent storage.
  const newLaunch = fakeFirstLaunch(new Map(), new Map());
  installStartupRecovery({ hostWindow: newLaunch.hostWindow });
  newLaunch.advance(1000);
  assert.equal(newLaunch.overlay().children[1].textContent, 'Подготовка приложения…');
});

test('hung blank first run reloads the main WebView once, then offers manual retry', () => {
  const storage = new Map([['some-unrelated-session-data', 'preserve']]);
  const first = fakeFirstLaunch(storage);
  installStartupRecovery({ hostWindow: first.hostWindow });
  first.advance(10001);
  assert.equal(first.reloads(), 1);
  assert.equal(storage.get('termian:startup:reload-attempted'), '1');
  assert.equal(storage.get('some-unrelated-session-data'), 'preserve');

  // Page reload creates a new document but shares sessionStorage.
  const second = fakeFirstLaunch(storage);
  installStartupRecovery({ hostWindow: second.hostWindow });
  second.advance(15000);
  assert.equal(second.reloads(), 0, 'Do not enter an automatic reload loop');
  assert.equal(second.overlay().children[1].textContent,
    'Интерфейс не загрузился. Данные сохранены.');
  const retry = second.overlay().children.find(child => child.textContent === 'Повторить загрузку');
  assert.ok(retry, 'Show a manual recovery option rather than a black screen');
  retry.listeners.get('click')();
  assert.equal(second.reloads(), 1);
  assert.equal(storage.has('termian:startup:reload-attempted'), false);
});

test('startup recovery is attached only to the primary Tauri window', () => {
  const bridge = fs.readFileSync(new URL('../src/original-bridge.js', import.meta.url), 'utf8');
  assert.match(bridge, /import \{ installStartupRecovery \} from '\.\/bridge\/startup-recovery\.js'/);
  assert.match(bridge, /if \(isPrimary\) installStartupRecovery\(\)/);
  assert.ok(bridge.indexOf('if (isPrimary) installStartupRecovery()') <
    bridge.indexOf('const appWindow = getCurrentWindow()'),
  'Install recovery before first native API access');
  assert.doesNotMatch(bridge, /if \(isBackground\) installStartupRecovery\(\)/);
});
