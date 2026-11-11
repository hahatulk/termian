/*
 * Recovered Termius UI module: mailto-0aef9dd0.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { b5 as m, bj as i, Ai as u, bX as $, gG as d, Aj as p, X as S } from "./reconnectSaga-3b11f62e.js";
const I = ({ email: o, subject: e = "", body: s = "", cc: t = [] }) => {
  if (!o) throw new Error("Cannot mailto without correct email");
  const n = encodeURIComponent(e), l = encodeURIComponent(s), c = `&cc=${t.join(";")}`, r = `mailto:${o}?subject=${n}&body=${l}${c}`;
  m({ title: `Email to ${o}`, url: r });
}, f = ({ userId: o, planType: e, username: s, currentPeriodExpiration: t }) => {
  const n = S(), l = o ? `
AccountID: ${o}` : "", c = e ? `
Plan: ${e}` : "", r = s ? `
Email: ${s}` : "", a = t != null && t.until ? `
Valid Until: ${t.until}` : "";
  return `

----------
App Information:
Platform: Desktop
OS Version: ${n.osVersion}
App Version: ${n.appVersion}
App Source: ${n.installSource}${l}${r}${c}${a}
`;
}, T = () => {
  const o = i(u), e = i($), s = i(d), t = i(p), n = f({ planType: o, userId: e, username: s, currentPeriodExpiration: t });
  return (c) => {
    const r = `${c.body || ""}${n}`;
    return I({ ...c, body: r });
  };
}, U = () => {
  const o = T();
  return () => o({ subject: "Need help with Termius", email: "support@termius.com" });
};
export {
  I as m,
  U as u
};
