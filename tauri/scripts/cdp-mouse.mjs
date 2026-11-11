// Developer helper: real mouse click at viewport coordinates.
// Usage: node scripts/cdp-mouse.mjs <url substring> <x> <y> [clickCount]
const [filter, x, y, count = '1'] = process.argv.slice(2);
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
for (let i = 1; i <= Number(count); i++) {
  const base = {x: Number(x), y: Number(y), button: 'left', clickCount: i};
  await send('Input.dispatchMouseEvent', {type: 'mousePressed', ...base});
  await send('Input.dispatchMouseEvent', {type: 'mouseReleased', ...base});
}
ws.close();
