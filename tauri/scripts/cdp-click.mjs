// Developer helper: click the first visible button whose text matches.
// Usage: node scripts/cdp-click.mjs <url substring> <button text>
const [filter, label] = process.argv.slice(2);
const expression = `(() => { const b = [...document.querySelectorAll('button,[role=button],a')].find(x => x.offsetParent && x.textContent.trim() === ${JSON.stringify(label)}); if (!b) return 'not found'; b.click(); return 'clicked'; })()`;
process.argv = [process.argv[0], process.argv[1], '--page=' + filter, expression];
await import('./cdp.mjs');
