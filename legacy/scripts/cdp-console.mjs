// Developer helper: reload a Termian webview and print its console output.
// Usage: node scripts/cdp-console.mjs <url substring> [seconds] [--no-reload]
const [filter = 'index.html', seconds = '15'] = process.argv.slice(2).filter(a => !a.startsWith('--'));
const reload = !process.argv.includes('--no-reload');
const pages = await (await fetch(process.env.CDP_URL || 'http://127.0.0.1:9298/json')).json();
const page = pages.find(p => p.type === 'page' && p.url.includes(filter));
if (!page) throw new Error('No page matching ' + filter);
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { ws.addEventListener('open', resolve, {once: true}); ws.addEventListener('error', reject, {once: true}); });
let id = 0;
const send = (method, params = {}) => ws.send(JSON.stringify({id: ++id, method, params}));
const text = arg => arg.value !== undefined ? (typeof arg.value === 'string' ? arg.value : JSON.stringify(arg.value)) : (arg.description ?? arg.type);
ws.addEventListener('message', event => {
  const m = JSON.parse(event.data);
  if (m.method === 'Runtime.consoleAPICalled') console.log(`[${m.params.type}]`, m.params.args.map(text).join(' ').slice(0, 1500));
  if (m.method === 'Runtime.exceptionThrown') console.log('[exception]', (m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text).slice(0, 1500));
  if (m.method === 'Log.entryAdded') console.log(`[log:${m.params.entry.level}]`, m.params.entry.text.slice(0, 800), m.params.entry.url ?? '');
});
send('Runtime.enable'); send('Log.enable');
if (reload) send('Page.reload', {ignoreCache: true});
setTimeout(() => { ws.close(); process.exit(0); }, Number(seconds) * 1000);
