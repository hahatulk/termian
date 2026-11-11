/*
 * Recovered Termius UI module: applyTeamVaultTargetState-377d7128.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bt as a, cX as l, br as r, bs as n, bu as t, by as d, iS as c, l3 as h } from "./reconnectSaga-3b11f62e.js";
function p({ mode: e, variant: i }) {
  const o = i === "shareEntitiesDialog" ? "column" : "row", s = a(d, { width: "12px", height: "12px" });
  return a(l, { children: e === "credentials_sharing" ? r(n, { direction: o, gap: "5px", children: [a(t, { variant: "body2", color: "main", children: "Username, passwords and keys will be shared in the" }), r(n, { direction: "row", alignItems: "center", gap: "5px", children: [s, a(t, { variant: "body2", color: "main", bold: true, children: "Team vault" })] })] }) : r(n, { direction: o, gap: "5px", children: [a(t, { variant: "body2", color: "main", children: "Credentials are not shared and stored in" }), r(n, { direction: "row", alignItems: "center", gap: "5px", children: [s, a(t, { variant: "body2", color: "main", bold: true, children: "Personal vaults" })] })] }) });
}
function u(e, i) {
  return { ...e ? c(e) : h, credentials_mode: i === "credentials_sharing" ? "credentials_sharing" : "no_credentials_sharing" };
}
export {
  p as S,
  u as a
};
