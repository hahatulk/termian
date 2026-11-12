/*
 * Recovered Termius UI module: LogOut-fc16f3ce.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as s, cN as f, bq as S, bj as v, y3 as b, bt as e, pm as K, bO as k, br as w, xH as l, bi as B, y4 as D, ew as I, gW as R } from "./reconnectSaga-3b11f62e.js";
import { L as j, K as C } from "./LogoutMainScreen-2a4c75a2.js";
import { u as F } from "./fadeIn-d66fb556.js";
const q = s.forwardRef((p, d) => {
  const { closeDialog: o, ...h } = p, { classes: t } = L(), g = f(), m = S(), x = F(h), r = [...v(b)], y = r.length !== 0, [i, u] = s.useState(null), [a, n] = s.useState("mainScreen"), c = async () => {
    try {
      y && a !== "keepBiometricKeys" ? (m(D()), n("keepBiometricKeys")) : (await I(g, R, { keepSepKeys: !!i }), o());
    } catch {
    }
  };
  return e(K.div, { style: x, ref: d, children: e("div", { className: t.dialog, children: e(k.Fragment, { children: e("div", { className: t.root, children: w("div", { className: t.mainWrapper, children: [e(l, { in: a === "mainScreen", top: 40, alignItems: "center", children: e(j, { onBack: () => o(), onLogOut: c, leftButtonTitle: "Cancel" }) }), e(l, { in: a === "keepBiometricKeys", top: 40, alignItems: "center", children: e(C, { onBack: () => n("mainScreen"), onLogOut: c, sepKeys: r, keepKeys: i, setKeepKeys: u }) })] }) }) }) }) });
}), L = B()({ dialog: { "--text-color": "var(--dark-grey-1)", "--close-color": "var(--light-grey-1)", "--border-color": "var(--light-grey-5)", ".termius-dark-theme &": { "--text-color": "var(--white)", "--close-color": "var(--dark-grey-7)", "--border-color": "var(--dark-grey-2)" }, overflow: "hidden", width: "720px", height: "100%", borderRadius: "20px", boxShadow: "0px 5px 15px rgba(0, 0, 0, 0.15)", position: "relative", display: "flex", flexDirection: "column", "@media (max-height: 460px) and (max-width:730px)": { borderRadius: "0" } }, root: { backgroundColor: "var(--foreground)", color: "var(--text-color)", width: "100%", height: "100%", display: "flex", flex: 1, flexDirection: "column", alignItems: "stretch", justifyContent: "space-between", gap: "20px" }, mainWrapper: { height: "540px", "@media (max-height: 520px)": { height: "480px" }, "@media (max-height: 480px)": { height: "460px" }, "@media (max-height: 460px)": { height: "100vh" } } });
export {
  q as default
};
