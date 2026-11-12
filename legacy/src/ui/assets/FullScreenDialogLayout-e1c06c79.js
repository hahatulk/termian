/*
 * Recovered Termius UI module: FullScreenDialogLayout-e1c06c79.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as x, br as s, bO as w, bt as t } from "./reconnectSaga-3b11f62e.js";
const v = (i) => {
  const { top: a, left: n, right: r, center: d, bottom: c, className: m, hideOverflow: h, fixedFooter: o } = i, { classes: e, cx: l } = u(), f = s(w.Fragment, { children: [a && t("div", { className: e.top, children: a }), (n || r) && s("div", { className: l(e.columns, i.classes.columns), children: [t("div", { className: l(e.left, i.classes.left), children: n }), t("div", { className: l(e.right, i.classes.right), children: r })] }), d && t("div", { className: l(e.center, i.classes.center), children: d })] });
  return s("div", { className: l(e.root, { [e.hideOverflow]: h }, m), children: [o ? t("div", { className: l({ [e.hideOverflow]: h, [e.scrollableArea]: o }), children: f }) : f, c && t("div", { className: l(e.bottom, { [e.fixedFooter]: o }), children: c })] });
};
v.defaultProps = { className: "", top: null, left: null, right: null, bottom: null, center: null, classes: {}, hideOverflow: false, fixedFooter: false };
const u = x()({ root: { width: "100%", height: "100%", display: "flex", position: "relative", flexDirection: "column", alignItems: "center", overflowY: "overlay", overflowX: "hidden" }, scrollableArea: { width: "100%", height: "100%", overflowX: "hidden", overflowY: "scroll", display: "flex", flexDirection: "column" }, hideOverflow: { overflow: "hidden " }, columns: { display: "flex", flexDirection: "row", padding: 0, width: "100%", flexWrap: "wrap", flex: "1", alignItems: "center", justifyContent: "center", position: "relative", maxWidth: "1200px", overflowY: "auto", overflowX: "hidden", "@media only screen and (min-height: 681px)": { padding: "30px 0 70px" } }, top: { width: "100%" }, left: { width: "50%", "@media only screen and (max-width: 900px)": { width: "100%" } }, right: { width: "50%", "@media only screen and (max-width: 900px)": { width: "100%" } }, center: { width: "100%", flex: "1", height: "100%", display: "flex", alignItems: "center", justifyContent: "center", minHeight: "300px" }, bottom: { width: "100%" }, fixedFooter: { marginTop: "auto" } });
export {
  v as F
};
