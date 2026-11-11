/*
 * Recovered Termius UI module: SftpSettings-54c64fdb.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as p, bt as e, bs as y, bF as f, bq as S, bj as d, br as a, bP as b, ms as v, dl as T, en as c, e6 as A, ej as k, vJ as w, vK as C, fh as N, Ca as F } from "./reconnectSaga-3b11f62e.js";
import { M as j, c as E, e as h } from "./index-bc78ea9d.js";
import { S as g, a as P } from "./SettingsSection-bc44d6bf.js";
import { S as R } from "./SettingsTabContent-ce30872f.js";
function O() {
  const { classes: i } = B();
  return e(g, { children: e(y, { alignItems: "center", justifyContent: "center", children: e(f, { title: "No file type associations yet", text: "Apps you choose using the “Open with…” option to open files via SFTP will appear here.", classNameRoot: i.card, icon: e(j, { height: "25px", width: "25px" }) }) }) });
}
const B = p()({ card: { maxWidth: "310px" } }), D = E();
function I(i) {
  const { className: o } = i, { classes: n } = z(), r = S(), m = d(h), l = (s, t = "") => {
    t && s.appPath !== t && (r(w({ ...s, appPath: t })), r(C({ onScreen: "Settings" })));
  }, x = async (s) => {
    const { filePaths: t } = await N.showOpenDialog({ filters: [{ name: "Applications", extensions: [D] }] });
    l(s, t[0]);
  }, u = async (s) => {
    r(F(s));
  };
  return a("div", { className: b(o, n.extensionsTable), children: [a("div", { className: n.header, children: [e("span", { children: "File Type" }), e("span", { children: "Open With" })] }), m.map((s) => a("div", { className: n.row, children: [a("span", { children: [".", s.type] }), e(v, { defaultValue: s.appPath, onBlur: (t) => l(s, t.target.value) }), a(T, { margin: "sm", className: n.actions, children: [e(c, { className: n.edit, onClick: () => x(s), children: e(A, {}) }), e(c, { color: "error", onClick: () => u(s), children: e(k, {}) })] })] }, s.appPath.concat(s.type)))] });
}
const z = p()({ extensionsTable: { "--bg": "var(--light-grey-5)", "--text-color": "var(--dark-grey-1)", ".termius-dark-theme &": { "--bg": "var(--dark-grey-2)", "--text-color": "white" }, color: "var(--text-color)", background: "var(--bg)", padding: "10px", borderRadius: "10px", marginTop: "15px" }, header: { display: "grid", gridTemplateColumns: "25% 1fr", lineHeight: "40px", padding: "0 10px" }, row: { "--row-bg": "var(--light-grey-7)", ".termius-dark-theme &": { "--row-bg": "var(--dark-grey-3)" }, display: "grid", gridTemplateColumns: "25% 45% 1fr", alignItems: "center", background: "var(--row-bg)", borderRadius: "10px", height: "40px", lineHeight: "40px", marginTop: "10px", padding: "0 10px" }, actions: { marginLeft: "auto", "& button": { height: "28px", width: "28px", padding: 0, display: "flex", fontSize: "20px", "& > svg": { fontSize: "inherit" } } }, edit: { color: "var(--text-color)" } });
function $({ className: i }) {
  const o = d(h);
  return e(R, { className: i, children: o.length ? a(g, { children: [e(P, { children: "File type associations" }), e(I, {})] }) : e(O, {}) });
}
export {
  $ as SftpSettings
};
