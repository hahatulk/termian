/*
 * Recovered Termius UI module: DialogPanel-fd3d3785.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as g, br as n, bs as r, bt as h, bi as m } from "./reconnectSaga-3b11f62e.js";
const b = g.forwardRef((a, e) => {
  var d;
  const { size: o, slotProps: t } = a, { width: c, height: l } = o, { classes: i, cx: x } = s();
  return n(r, { component: "div", role: "dialog", tabIndex: 0, justifyContent: "space-between", className: i.dialogPanel, ref: e, style: { maxWidth: c, maxHeight: l }, children: [h("div", { className: x(i.dialogContent, (d = t == null ? void 0 : t.content) == null ? void 0 : d.className), children: a.content }), a.actions] });
});
function u(a) {
  const { classes: e } = s();
  return n(r, { direction: "row", justifyContent: a.start != null ? "space-between" : "end", width: "100%", className: e.dialogActions, children: [a.start, a.end] });
}
const s = m()((a, e, o) => ({ dialogPanel: { width: "100vw", height: "100vh", borderRadius: "20px", backgroundColor: "var(--foreground)", color: "var(--main)", "@media (max-height: 460px) and (max-width:730px)": { borderRadius: 0 } }, dialogContent: { flexGrow: 1, padding: "40px", position: "relative", overflow: "hidden", "@media (max-height: 500px)": { paddingTop: "11px", paddingBottom: "11px" }, [`&:has(~ .${o.dialogActions})`]: { paddingBottom: "20px" } }, dialogActions: { "--border-color": "var(--card-b)", ".termius-dark-theme &": { "--border-color": "var(--card-c)" }, padding: "20px", borderTop: "1px solid var(--border-color)", "@media (max-height: 500px)": { paddingTop: "11px", paddingBottom: "11px" } } }));
export {
  b as D,
  u as a
};
