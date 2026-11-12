/*
 * Recovered Termius UI module: TeamMemberSubscriptionExpired-ffee3c44.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as u, bt as e, fu as S, r as l, br as r, cv as d, bu as n, dE as C, bh as T, bj as h, cG as N, cN as I, fH as v, X as R, ew as E, gW as D, pm as O, dl as L } from "./reconnectSaga-3b11f62e.js";
import { u as _ } from "./fadeIn-d66fb556.js";
import { m as j } from "./mailto-0aef9dd0.js";
import { d as U } from "./Done-8ecd181f.js";
import { d as A } from "./ErrorOutline-9dd3d7a0.js";
const M = { exit: 300, enter: 300 }, c = ({ isVisible: a, children: o }) => {
  const { classes: t } = P();
  return e(S, { in: a, timeout: M, children: e("div", { className: t.item, children: o }) });
}, P = u()({ item: { position: "absolute", left: 0, width: "100%", display: "flex", justifyContent: "center", alignItems: "center" } }), W = "Copy team owner's email", X = "Copy team owner's email", B = "Team owner's email is copied", V = "Something went wrong!";
function $({ className: a, email: o }) {
  const { classes: t } = F(), [i, s] = l.useState("default");
  return r(d, { color: "accent", className: a, onClick: () => {
    s("loading"), T.writeText(o), setTimeout(() => {
      s("success");
    }, 800), setTimeout(() => {
      s("default");
    }, 4e3);
  }, fullWidth: true, children: [e(c, { isVisible: i === "default", children: e(n, { color: "inherit", children: W }) }), r(c, { isVisible: i === "success", children: [e(U, { className: t.icon }), e(n, { color: "inherit", children: B })] }), r(c, { isVisible: i === "error", children: [e(A, { className: t.icon }), e(n, { color: "inherit", children: V })] }), r(c, { isVisible: i === "loading", children: [e(C, { className: t.icon, size: 24, classes: { colorPrimary: t.progressColor } }), e(n, { color: "inherit", children: X })] })] });
}
const F = u()({ icon: { marginRight: "5px", color: "var(--light-grey-1)", ".termius-dark-theme &": { color: "var(--white)" } }, progressColor: { color: "var(--light-grey-1)", ".termius-dark-theme &": { color: "var(--white)" } } }), Y = "" + new URL("oops@1x-db6bad42.webp", import.meta.url).href, G = "" + new URL("oops@2x-1b87803e.webp", import.meta.url).href, q = "" + new URL("oopsDark@1x-7a9cd92e.webp", import.meta.url).href, z = "" + new URL("oopsDark@2x-be4a1b5e.webp", import.meta.url).href, H = (a) => {
  const o = h(N), [t, i] = o ? a.dark : a.light;
  return { src: t, srcSet: `${i} 2x` };
}, se = (a) => {
  const { closeDialog: o, ownerAddress: t, ...i } = a, { classes: s } = K(), m = I(), g = _(i), [p, x] = l.useState(false), f = h(v), w = R().platform === "linux", b = H({ light: [Y, G], dark: [q, z] }), y = l.useCallback(() => {
    j({ email: t });
  }, [t]), k = l.useCallback(async () => {
    x(true), await E(m, D, { keepSepKeys: false }), o();
  }, [o, m]);
  return e(O.div, { style: g, className: s.dialog, children: r("div", { className: s.root, children: [r("div", { className: s.content, children: [e(n, { variant: "h2", color: "main", align: "left", children: "Your team subscription has expired" }), r(n, { color: "main", align: "left", children: ["Termius Team subscription for ", f, " has expired. To continue working as a team, the owner of your team has to renew your team's subscription."] }), r(L, { margin: "lg", className: s.buttonRow, children: [w ? e($, { email: t }) : e(d, { onClick: y, color: "accent", fullWidth: true, children: "Email team owner" }), e(d, { onClick: k, fullWidth: true, progress: p, disabled: p, children: "Log out" })] })] }), e("div", { className: s.image, children: e("img", { ...b, alt: "" }) })] }) });
}, K = u()({ dialog: { width: "100%", height: "100%", display: "flex" }, root: { width: "calc(100% - 80px)", margin: "auto auto 40px", display: "flex", alignItems: "center", justifyContent: "space-between", gap: 40, background: "var(--background)", padding: "65px 80px", flexWrap: "wrap-reverse", borderRadius: 20, "@media screen and (max-width: 1263px)": { justifyContent: "center" }, "@media screen and (max-height: 542px)": { gap: 5, padding: "32px 40px" } }, content: { width: "700px", display: "flex", flexDirection: "column", alignItems: "start", gap: 20, "@media screen and (max-width: 1263px)": { width: "100%" } }, buttonRow: { width: "100%", marginTop: 20 }, image: { width: "284px", "& img": { width: "100%" } } });
export {
  se as default
};
