/*
 * Recovered Termius UI module: WarnAboutSSHidDeviceCount-09c75630.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { u as d } from "./useRequest-2d237c57.js";
import { bj as S, An as v, cN as u, ew as n, Ao as g, Ap as w, Aq as x, bi as m, br as i, cX as I, bu as c, bt as r, Ar as b } from "./reconnectSaga-3b11f62e.js";
function h(e) {
  const s = S(v), a = u(), o = d(async (t = void 0) => {
    (t === void 0 || t.device.id === (s == null ? void 0 : s.device.id)) && await n(a, g, { wizard_id: e.wizardID });
  }, { manual: true, onBefore: () => {
    var t;
    (t = e == null ? void 0 : e.onStartGenerating) == null || t.call(e);
  } }), l = o.loading ? o.params[0] : void 0;
  return { request: o, deviceInProgress: l };
}
function A() {
  const e = u(), s = d(async (o) => {
    await n(e, w, o.device), await n(e, x);
  }, { manual: true }), a = s.loading ? s.params[0] : void 0;
  return { request: s, deviceInProgress: a };
}
function H() {
  const { classes: e } = p();
  return i(I, { className: e.warnItem, children: [i(c, { className: e.textWithIcon, bold: true, color: "main", children: [r(b, { className: e.warnIcon }), "It is recommended to use at least two devices"] }), r(c, { color: "main", children: "Log in on multiple devices to ensure access in case one is lost or stolen." })] });
}
const p = m()({ warnItem: { width: "100%", padding: "15px", borderRadius: "15px", backgroundColor: "var(--yellow-a15)" }, textWithIcon: { position: "relative", textIndent: "24px" }, warnIcon: { position: "absolute", color: "var(--yellow)", left: "0px", top: "2px" } });
export {
  H as W,
  h as a,
  A as u
};
