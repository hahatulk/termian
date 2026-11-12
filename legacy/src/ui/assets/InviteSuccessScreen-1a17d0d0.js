/*
 * Recovered Termius UI module: InviteSuccessScreen-1a17d0d0.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as s, bt as e, bO as o, br as t, bs as n, bx as r, za as l, bu as h, cv as d } from "./reconnectSaga-3b11f62e.js";
const g = (i) => {
  const { onClose: c } = i, { classes: a } = p();
  return e(o.Fragment, { children: t(n, { direction: "column", alignItems: "center", width: "100%", height: "100%", justifyContent: "space-between", children: [e(n, { direction: "column", alignItems: "center", width: "100%", height: "466px", paddingTop: "40px", gap: "55px", justifyContent: "center", className: a.contentWrapper, children: t(n, { direction: "column", alignItems: "center", gap: "30px", children: [e(r, { icon: e(l, {}), size: 60 }), e(h, { variant: "h2", color: "main", align: "center", children: "The invites have been sent!" })] }) }), e(n, { width: "100%", display: "flex", flexDirection: "column", alignItems: "end", padding: "20px", children: e(d, { onClick: c, color: "accent", children: "Done" }) })] }) });
}, p = s()({ contentWrapper: { height: "466px", borderBottom: "1px solid var(--card-b)", "@media (max-height: 520px)": { height: "85%" } } });
export {
  g as I
};
