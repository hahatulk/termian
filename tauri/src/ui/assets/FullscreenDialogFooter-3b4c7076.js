/*
 * Recovered Termius UI module: FullscreenDialogFooter-3b4c7076.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as l, bt as o } from "./reconnectSaga-3b11f62e.js";
const f = (e) => {
  const { className: n, children: s, onTransitionEnd: r = d, innerRef: a = null } = e, { classes: t, cx: i } = c();
  return o("div", { className: i(t.root, n), onTransitionEnd: r, ref: a, children: o("div", { className: t.footerContent, children: s }) });
}, c = l()({ root: { width: "100%", backgroundColor: "var(--main-bg)", display: "flex", justifyContent: "center" }, footerContent: { maxWidth: "380px", width: "100%", padding: "30px 0" } }), d = () => {
};
export {
  f as F
};
