/*
 * Recovered Termius UI module: TeamOwnerSubscriptionExpired-c8e362c5.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bj as d, cG as w, cN as b, r as c, gG as f, ew as k, gW as y, bt as a, pm as S, br as o, bu as l, dl as R, pE as D, cv as I, bi as v, x1 as L } from "./reconnectSaga-3b11f62e.js";
import { L as N } from "./LinkButton-03e176fc.js";
import { u as O } from "./fadeIn-d66fb556.js";
const j = "" + new URL("oops@1x-db6bad42.webp", import.meta.url).href, B = "" + new URL("oops@2x-1b87803e.webp", import.meta.url).href, C = "" + new URL("oopsDark@1x-7a9cd92e.webp", import.meta.url).href, T = "" + new URL("oopsDark@2x-be4a1b5e.webp", import.meta.url).href, U = (t) => {
  const s = d(w), [r, e] = s ? t.dark : t.light;
  return { src: r, srcSet: `${e} 2x` };
}, G = (t) => {
  const { closeDialog: s, ...r } = t, { classes: e } = W(), n = b(), p = O(r), [i, m] = c.useState(false), u = d(f), g = U({ light: [j, B], dark: [C, T] }), h = async () => {
    L({ username: u, plan: "team" });
  }, x = c.useCallback(async () => {
    m(true), await k(n, y, { keepSepKeys: false }), s();
  }, [s, n]);
  return a(S.div, { style: p, className: e.dialog, children: o("div", { className: e.root, children: [o("div", { className: e.content, children: [a(l, { variant: "h2", color: "main", align: "left", children: "Your subscription has expired" }), o(l, { color: "main", align: "left", children: ["Don't worry. Your data is secured in an encrypted cloud vault.", a("br", {}), "Renew your subscription to regain full access."] }), o(R, { margin: "lg", className: e.buttonRow, children: [a(N, { onClick: h, color: "accent", fullWidth: true, rightIcon: a(D, { className: e.icon }), children: "Renew now" }), a(I, { onClick: x, fullWidth: true, progress: i, disabled: i, children: "Log out" })] })] }), a("div", { className: e.image, children: a("img", { ...g, alt: "" }) })] }) });
}, W = v()({ dialog: { width: "100%", height: "100%", display: "flex" }, root: { width: "calc(100% - 80px)", margin: "auto auto 40px", display: "flex", alignItems: "center", justifyContent: "space-between", gap: 40, background: "var(--background)", padding: "65px 80px", flexWrap: "wrap-reverse", borderRadius: 20, "@media screen and (max-width: 1263px)": { justifyContent: "center" }, "@media screen and (max-height: 542px)": { gap: 5, padding: "32px 40px" } }, content: { width: "700px", display: "flex", flexDirection: "column", alignItems: "start", gap: 20, "@media screen and (max-width: 1263px)": { width: "100%" } }, buttonRow: { width: "100%", marginTop: 20 }, image: { width: "284px", "& img": { width: "100%" } }, icon: { width: "14px", height: "14px" } });
export {
  G as default
};
