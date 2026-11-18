// Developer helper: type text into the focused terminal and press Enter.
// Usage: node scripts/cdp-type.mjs <url substring> <text>
const [filter, text] = process.argv.slice(2);
const pages = await (await fetch(process.env.CDP_URL || 'http://127.0.0.1:9298/json')).json();
const page = pages.find(p => p.type === 'page' && p.url.includes(filter));
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise(resolve => ws.addEventListener('open', resolve, {once: true}));
let id = 0;
const send = (method, params) => new Promise(resolve => {
  const n = ++id;
  const listener = event => { const m = JSON.parse(event.data); if (m.id === n) { ws.removeEventListener('message', listener); resolve(m); } };
  ws.addEventListener('message', listener);
  ws.send(JSON.stringify({id: n, method, params}));
});
await send('Runtime.evaluate', {expression: "document.querySelector('.xterm-helper-textarea')?.focus()"});
await send('Input.insertText', {text});
await send('Input.dispatchKeyEvent', {type: 'keyDown', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13, text: '\r'});
await send('Input.dispatchKeyEvent', {type: 'keyUp', key: 'Enter', code: 'Enter', windowsVirtualKeyCode: 13});
ws.close();
