/*
 * Recovered Termius UI module: index-bc78ea9d.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as o, hc as a, X as c, l5 as m, dM as l } from "./reconnectSaga-3b11f62e.js";
const f = (e, t) => o.createElement("svg", { width: "14px", height: "12px", viewBox: "0 0 20 20", xmlns: "http://www.w3.org/2000/svg", xmlnsXlink: "http://www.w3.org/1999/xlink", ref: t, ...e }, o.createElement("g", { id: "V3-assets", stroke: "none", strokeWidth: 1, fill: "none", fillRule: "evenodd" }, o.createElement("g", { id: "Assets", transform: "translate(-401.000000, -490.000000)", fillRule: "nonzero", fill: "currentColor" }, o.createElement("g", { id: "sftp", transform: "translate(401.000000, 490.000000)" }, o.createElement("g", { id: "Layer_1" }, o.createElement("g", { id: "Page-1" }, o.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M8.28495 3.87276H18.5551C19.353 3.87276 20 4.51307 20 5.30286V6.6195H0.0274875V4.70515V3.87276V3.44735C0.0274875 2.64792 0.682139 2 1.48952 2H6.26154C6.54752 2 6.81882 2.12597 7.00144 2.34391L8.28495 3.87276ZM0.321517 7.3685H19.6508C19.8285 7.3685 19.9723 7.51104 19.9723 7.68672V17.7623C19.9723 18.3355 19.503 18.8 18.9239 18.8H1.04826C0.469463 18.8 0 18.3355 0 17.7623V7.68672C0 7.51104 0.143846 7.3685 0.321517 7.3685Z" }))))))), p = o.forwardRef(f), w = o.memo(p), C = a((e) => e.sftp.fileTypeAppAssociations, (e) => e || []), u = a((e) => e.transferStreams, (e) => e || []);
function x(e, t) {
  var r;
  return (r = e.find((n) => {
    var s;
    return ((s = n[1]) == null ? void 0 : s.sourcePath) === t;
  })) == null ? void 0 : r[1];
}
const y = a((e) => e.changedSftpFiles, (e) => e || []);
function T(e, t) {
  return (e == null ? void 0 : e.filter((r) => {
    var n;
    return ((n = r[1]) == null ? void 0 : n.sftpContainerId) === t;
  })) || [];
}
const E = a(u, (e) => e.filter(([t, r]) => {
  var n, s;
  return !((n = r.source) != null && n.isTemp) && !((s = r.destination) != null && s.isTemp);
}));
function v(e, t) {
  return e.filter(([r, n]) => {
    var s, i;
    return ((s = n.source) == null ? void 0 : s.sessionId) === t && ((i = n.destination) == null ? void 0 : i.isTemp);
  });
}
const d = m({ year: "numeric", month: "numeric", day: "numeric", hour: "numeric", minute: "numeric" });
function F(e) {
  try {
    return g(S(Math.floor(e)));
  } catch {
    return "";
  }
}
function g(e) {
  return d.format(e);
}
function S(e) {
  return new Date(e * 1e3);
}
const M = (e) => (t, r) => l(t, e) - l(r, e), V = () => c().platform === "mac" ? "app" : c().platform === "win" ? "exe" : "";
export {
  w as M,
  v as a,
  x as b,
  V as c,
  T as d,
  C as e,
  F as f,
  M as g,
  y as h,
  E as n,
  u as t
};
