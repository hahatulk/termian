/*
 * Recovered Termius UI module: KeyboardKey-4328b011.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as t, bi as i, bt as l } from "./reconnectSaga-3b11f62e.js";
const c = (r, a) => t.createElement("svg", { width: 24, height: 11, viewBox: "0 0 24 11", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: a, ...r }, t.createElement("path", { d: "M0.890718 10.4109C3.49947 10.1501 5.86479 8.76459 7.36827 6.61675L12 0L16.6317 6.61675C18.1352 8.76458 20.5005 10.1501 23.1093 10.4109L24 10.5H0L0.890718 10.4109Z", fill: "inherit" })), d = t.forwardRef(c), m = t.memo(d);
function x(r) {
  const { className: a, children: e, ...n } = r, { classes: s, cx: o } = p();
  return l("div", { className: o(s.keyboardKey, { [s.smallKey]: typeof e != "string" || (e == null ? void 0 : e.length) === 1 }, a), ...n, children: e });
}
const p = i()({ keyboardKey: { borderRadius: "4px", lineHeight: "22px", minWidth: "22px", height: "22px", display: "flex", alignItems: "center", justifyContent: "center", textAlign: "center", padding: "0 6px", textTransform: "uppercase", fontSize: "8px", fontWeight: 500, color: "var(--secondary)", background: "var(--light-grey-5)", ".termius-dark-theme &": { background: "var(--dark-grey-5)" } }, smallKey: { padding: "0", width: "22px", fontSize: "14px" } });
export {
  x as K,
  m as M
};
