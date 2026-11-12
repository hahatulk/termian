/*
 * Recovered Termius UI module: PageWrapper-a7d0cf6b.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { cA as r, bt as s, bP as i } from "./reconnectSaga-3b11f62e.js";
const t = (e) => {
  var a;
  return s("div", { id: e.id, className: i(e.className, (a = e.classes) == null ? void 0 : a.wrapper, { "termius-dark-theme": e.dark }), children: e.children });
}, l = (e, a) => ({ wrapper: { width: "100%", height: "100%", overflow: "hidden", display: "flex", flexDirection: "column", backgroundColor: "var(--main-bg)", ".termius-dark-theme &": { backgroundColor: "inherit" } } }), d = r(t, l, { name: "PageWrapper" });
export {
  d as P
};
