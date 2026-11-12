/*
 * Recovered Termius UI module: FlowControl-a7826665.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as r, bt as e, cv as c, yI as d, vI as m } from "./reconnectSaga-3b11f62e.js";
const h = (s) => {
  const { children: a } = s, { classes: t } = n();
  return e("div", { className: t.root, children: a });
}, x = (s) => {
  const { onClick: a, className: t, size: o = "medium" } = s, { cx: l, classes: i } = n();
  return e(c, { size: o, onClick: a, variant: "icon", "aria-label": "Back button", className: l(i.backShift, t), children: e(d, { width: 16, height: 12, className: i.icon }) });
}, g = (s) => {
  const { onClick: a, className: t } = s, { classes: o } = n();
  return e(c, { size: "small", onClick: a, variant: "icon", className: t, children: e(m, { width: 18, height: 18, className: o.icon, "aria-label": "Close button" }) });
}, n = r()({ root: { position: "absolute", zIndex: 1, width: "100%", display: "flex", alignItems: "center", justifyContent: "space-between", padding: 10, paddingLeft: "36px", ".mac &": { top: "24px", paddingRight: "24px", "@media (max-height: 650px) or (max-width: 750px)": { top: "12px", paddingRight: "20px" } } }, icon: { color: "var(--secondary)" }, backShift: { "body:not(.settings) &": { marginTop: -2 } } });
export {
  x as B,
  g as C,
  h as F
};
