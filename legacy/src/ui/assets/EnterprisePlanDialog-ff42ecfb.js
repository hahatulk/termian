/*
 * Recovered Termius UI module: EnterprisePlanDialog-ff42ecfb.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as r, iD as o, bt as e, bs as i, xv as s } from "./reconnectSaga-3b11f62e.js";
function n(a) {
  const { classes: t } = l();
  return e(i, { className: t.root, children: e(i, { className: t.dialogContainer, children: e(s, { ...a, onOk: a.closeDialog }) }) });
}
const l = r()({ root: { alignItems: "center", justifyContent: "center", width: "100%", height: "100%" }, dialogContainer: { width: "720px", height: "540px", backgroundColor: "var(--foreground)", borderRadius: "20px", position: "relative", transition: ".5s background-color ease", overflow: "hidden", "@media (max-height: 540px)": { height: "100%" }, "@media (max-width: 720px)": { borderRadius: "unset" }, "@media (max-height: 460px) and (max-width:730px)": { borderRadius: 0 } } }), h = o(n);
export {
  h as default
};
