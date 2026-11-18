/*
 * Recovered Termius UI module: FiltersButton-2e4a50c6.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as o, br as i, bt as c } from "./reconnectSaga-3b11f62e.js";
const p = (n) => {
  const { children: s, icon: e, className: a, ...r } = n, { classes: t, cx: l } = d();
  return i("button", { type: "button", className: l(t.button, a), ...r, children: [e && c("span", { className: t.icon, children: e }), s] });
};
p.defaultProps = { icon: null, className: "" };
const d = o()({ button: { display: "flex", alignItems: "center", border: "none", backgroundColor: "transparent", fontSize: "14px", height: "30px", lineHeight: "14px", fontWeight: 700, color: "var(--filters-header-color)", fill: "var(--filters-header-icon)", padding: "0 0 0 20px", textTransform: "none", letterSpacing: "normal", "&:hover": { color: "var(--accent)", fill: "var(--accent)", "& svg": { fill: "var(--accent)" } } }, icon: { "& svg": { fill: "inherit" }, display: "flex", alignItems: "center", width: "20px", paddingRight: "5px" } });
export {
  p as F
};
