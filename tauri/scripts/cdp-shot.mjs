// Developer helper: save a PNG screenshot of a Termian webview.
// Usage: node scripts/cdp-shot.mjs <url substring> <out.png>
import fs from 'node:fs';
const [filter = 'index.html', out = 'shot.png'] = process.argv.slice(2);
const pages = await (await fetch(process.env.CDP_URL || 'http://127.0.0.1:9298/json')).json();
const page = pages.find(p => p.type === 'page' && p.url.includes(filter));
if (!page) throw new Error('No page matching ' + filter);
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { ws.addEventListener('open', resolve, {once: true}); ws.addEventListener('error', reject, {once: true}); });
ws.addEventListener('message', event => {
  const m = JSON.parse(event.data);
  if (m.id !== 1) return;
  fs.writeFileSync(out, Buffer.from(m.result.data, 'base64'));
  ws.close();
});
ws.send(JSON.stringify({id: 1, method: 'Page.captureScreenshot', params: {format: 'png'}}));
