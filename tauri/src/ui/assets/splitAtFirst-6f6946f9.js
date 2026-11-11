/*
 * Recovered Termius UI module: splitAtFirst-6f6946f9.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
function s(i) {
  return i.length === 2;
}
function c(i, t) {
  const n = i.indexOf(t);
  return n === -1 ? [i] : [i.slice(0, n), i.slice(n + t.length)];
}
export {
  s as i,
  c as s
};
