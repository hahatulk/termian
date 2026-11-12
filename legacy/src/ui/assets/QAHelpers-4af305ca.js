/*
 * Recovered Termius UI module: QAHelpers-4af305ca.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bq as p, bt as e, br as i, bi as d, bu as o, cv as h, Cp as u, pp as S, Cq as g } from "./reconnectSaga-3b11f62e.js";
import { S as m, a as D } from "./SettingsSection-bc44d6bf.js";
import { S as b } from "./SettingsTabContent-ce30872f.js";
const n = ({ label: t, onClick: s, children: a }) => {
  const { classes: l } = c();
  return i("div", { className: l.container, children: [e(o, { fontSize: 12, children: t }), e(h, { size: "small", onClick: s, children: a })] });
}, r = ({ children: t }) => {
  const { classes: s } = c();
  return e(o, { fontSize: 14, bold: true, className: s.separator, children: t });
}, C = () => {
  const t = p();
  return e(b, { children: i(m, { children: [e(D, { children: "QA Helpers" }), e(r, { children: "Snippets panel" }), e(n, { label: "Set first launch day to yesterday", onClick: () => {
    t(u(Date.now() - S));
  }, children: "-24 hours" }), e(r, { children: "Deactivate Business trial" }), e(n, { label: "Generate test data", onClick: () => {
    t(g());
  }, children: "Generate (disable internet first)" })] }) });
}, c = d()({ container: { display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%", marginTop: 15 }, separator: { marginTop: "20px" } });
export {
  C as default
};
