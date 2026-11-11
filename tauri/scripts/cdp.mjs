// Developer helper: evaluate an expression in a running Termian webview through
// WebView2's remote debugging port (start the app with
// WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9298).
// Usage: node scripts/cdp.mjs [--page=<url substring>] <expression>
const args = process.argv.slice(2);
const pageArg = args.find(a => a.startsWith('--page='));
const expression = args.filter(a => a !== pageArg).join(' ');
const filter = pageArg ? pageArg.slice(7) : 'index.html';
const pages = await (await fetch(process.env.CDP_URL || 'http://127.0.0.1:9298/json')).json();
if (!expression) { console.log(pages.map(p => p.type + ' ' + p.url).join('\n')); process.exit(0); }
const page = pages.find(p => p.type === 'page' && p.url.includes(filter));
if (!page) throw new Error('No page matching ' + filter);
const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((resolve, reject) => { ws.addEventListener('open', resolve, {once: true}); ws.addEventListener('error', reject, {once: true}); });
ws.addEventListener('message', event => {
  const data = JSON.parse(event.data);
  if (data.id !== 1) return;
  const result = data.result?.exceptionDetails ? data.result.exceptionDetails : data.result?.result?.value ?? data.result?.result?.description ?? data.result;
  console.log(typeof result === 'string' ? result : JSON.stringify(result, null, 2).slice(0, 60000));
  ws.close();
});
ws.send(JSON.stringify({id: 1, method: 'Runtime.evaluate', params: {expression, returnByValue: true, awaitPromise: true}}));
