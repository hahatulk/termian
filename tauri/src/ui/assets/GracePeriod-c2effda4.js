/*
 * Recovered Termius UI module: GracePeriod-c2effda4.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bj as c, cG as x, bq as f, xb as w, cO as y, bt as e, pm as b, br as s, bu as l, dl as k, pE as D, cv as I, bi as T, wp as v, x4 as N, cl as S, xw as d, xx as L } from "./reconnectSaga-3b11f62e.js";
import { L as P } from "./LinkButton-03e176fc.js";
import { u as R } from "./fadeIn-d66fb556.js";
const B = "" + new URL("terminal@1x-70ffe6fb.webp", import.meta.url).href, U = "" + new URL("terminal@2x-ad665cb4.webp", import.meta.url).href, j = "" + new URL("terminalDark@1x-d2d5dfc8.webp", import.meta.url).href, G = "" + new URL("terminalDark@2x-9eb5ce78.webp", import.meta.url).href, C = (n) => {
  const o = c(x), [i, a] = o ? n.dark : n.light;
  return { src: i, srcSet: `${a} 2x` };
}, O = (n) => {
  const { closeDialog: o, ...i } = n, a = f(), { classes: t } = W(), m = R(i), r = c(w), p = c(y), u = C({ light: [B, U], dark: [j, G] }), g = async () => {
    v(), a(N({ buyNowButtonLocation: "Grace Period Notification", productID: p ? "team" : "premium", period: null, upgradeToProFunnelID: S.v4() })), a(d(r == null ? void 0 : r.current_period.until)), o();
  }, h = () => {
    a(d(r == null ? void 0 : r.current_period.until)), a(L()), o();
  };
  return e(b.div, { style: m, className: t.dialog, children: s("div", { className: t.root, children: [s("div", { className: t.content, children: [e(l, { variant: "h2", color: "main", align: "left", children: "Your payment failed" }), e(l, { color: "main", align: "left", children: "Don't worry. We'll try your payment again over the next few days. To keep your work uninterrupted, you may need to update your payment detail." }), s(k, { margin: "lg", className: t.buttonRow, children: [e(P, { onClick: g, color: "accent", fullWidth: true, rightIcon: e(D, { className: t.icon }), children: "Update details" }), e(I, { onClick: h, fullWidth: true, children: "I'll do it later" })] })] }), e("div", { className: t.image, children: e("img", { ...u, alt: "" }) })] }) });
}, W = T()({ dialog: { width: "100%", height: "100%", display: "flex" }, root: { width: "calc(100% - 80px)", margin: "auto auto 40px", display: "flex", alignItems: "center", justifyContent: "space-between", gap: 40, background: "var(--background)", padding: "65px 80px", flexWrap: "wrap-reverse", borderRadius: 20, "@media screen and (max-width: 1263px)": { justifyContent: "center" } }, content: { width: "700px", display: "flex", flexDirection: "column", alignItems: "start", gap: 20, "@media screen and (max-width: 1263px)": { width: "100%" } }, buttonRow: { width: "100%", marginTop: 20 }, image: { width: "284px" }, icon: { width: "14px", height: "14px" } });
export {
  O as default
};
