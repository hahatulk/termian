/*
 * Recovered Termius UI module: ShareEntities-f1e7cd1b.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as g, bt as e, pm as p, bO as m, br as x, xH as i, bz as u, x$ as f, bi as y, o6 as v } from "./reconnectSaga-3b11f62e.js";
import { S, a as C } from "./ShareEntitiesSuccessScreen-c24a2654.js";
import { u as b } from "./fadeIn-d66fb556.js";
import "./RadioOptionsList-6f3efe10.js";
import "./applyTeamVaultTargetState-377d7128.js";
const F = (s) => {
  const { closeDialog: n, teamTrialFunnelID: o, ...c } = s, { classes: r } = k(), l = b(c), [t, a] = g.useState("sharing"), h = () => {
    try {
      n();
    } catch (d) {
      a("error"), v(d, "Skip presenceEffect inviting failed");
    }
  };
  return e(p.div, { style: l, children: e("div", { className: r.dialog, children: e(m.Fragment, { children: e("div", { className: r.root, children: x("div", { className: r.mainWrapper, children: [e(i, { in: t === "sharing", justifyContent: "center", alignItems: "center", children: e(S, { teamTrialFunnelID: o, onClose: h, setContentType: a }) }), e(i, { in: t === "loading", justifyContent: "center", alignItems: "center", children: e("div", { className: r.spinnerCenter, children: e(u, { className: r.progressCircle, size: 205, thickness: 2 }) }) }), e(i, { in: t === "successSharing", justifyContent: "center", alignItems: "center", children: e(C, { onClose: () => n() }) }), e(i, { in: t === "errorSharing", justifyContent: "center", alignItems: "center", children: e(f, { onBack: () => a("sharing") }) })] }) }) }) }) });
}, k = y()({ dialog: { "--text-color": "var(--dark-grey-1)", "--close-color": "var(--light-grey-1)", "--border-color": "var(--light-grey-5)", ".termius-dark-theme &": { "--text-color": "var(--white)", "--close-color": "var(--dark-grey-7)", "--border-color": "var(--dark-grey-2)" }, overflow: "hidden", width: "720px", height: "100%", borderRadius: "20px", boxShadow: "0px 5px 15px rgba(0, 0, 0, 0.15)", position: "relative", display: "flex", flexDirection: "column", "@media (max-height: 460px) and (max-width:730px)": { borderRadius: "0" } }, root: { backgroundColor: "var(--foreground)", color: "var(--text-color)", width: "100%", height: "100%", display: "flex", flex: 1, flexDirection: "column", alignItems: "stretch", justifyContent: "space-between", gap: "20px" }, mainWrapper: { height: "540px", "@media (max-height: 520px)": { height: "480px" }, "@media (max-height: 480px)": { height: "460px" }, "@media (max-height: 460px)": { height: "100vh" } }, spinnerCenter: { width: "100%", height: "540px", display: "flex", justifyContent: "center", alignItems: "center", "@media (max-height: 540px)": { height: "100vh" } }, progressCircle: { color: "var(--accent)" } });
export {
  F as default
};
