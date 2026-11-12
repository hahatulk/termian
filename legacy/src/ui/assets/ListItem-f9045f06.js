/*
 * Recovered Termius UI module: ListItem-f9045f06.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { hI as h, hJ as E, hK as N, hM as o, r as l, hN as T, hO as R, xT as x, bt as v, hQ as L, hR as D, xU as I, xg as tt, xV as et, xW as st, br as _, xX as j, dI as ot } from "./reconnectSaga-3b11f62e.js";
function at(t) {
  return E("MuiListItem", t);
}
const nt = h("MuiListItem", ["root", "container", "focusVisible", "dense", "alignItemsFlexStart", "disabled", "divider", "gutters", "padding", "button", "secondaryAction", "selected"]), m = nt, it = h("MuiListItemButton", ["root", "focusVisible", "dense", "alignItemsFlexStart", "disabled", "divider", "gutters", "selected"]), rt = it;
function ct(t) {
  return E("MuiListItemSecondaryAction", t);
}
h("MuiListItemSecondaryAction", ["root", "disableGutters"]);
const lt = ["className"], dt = (t) => {
  const { disableGutters: e, classes: s } = t;
  return D({ root: ["root", e && "disableGutters"] }, ct, s);
}, pt = N("div", { name: "MuiListItemSecondaryAction", slot: "Root", overridesResolver: (t, e) => {
  const { ownerState: s } = t;
  return [e.root, s.disableGutters && e.disableGutters];
} })(({ ownerState: t }) => o({ position: "absolute", right: 16, top: "50%", transform: "translateY(-50%)" }, t.disableGutters && { right: 0 })), W = l.forwardRef(function(e, s) {
  const a = T({ props: e, name: "MuiListItemSecondaryAction" }), { className: n } = a, i = R(a, lt), d = l.useContext(x), p = o({}, a, { disableGutters: d.disableGutters }), b = dt(p);
  return v(pt, o({ className: L(b.root, n), ownerState: p, ref: s }, i));
});
W.muiName = "ListItemSecondaryAction";
const ut = W, mt = ["className"], bt = ["alignItems", "autoFocus", "button", "children", "className", "component", "components", "componentsProps", "ContainerComponent", "ContainerProps", "dense", "disabled", "disableGutters", "disablePadding", "divider", "focusVisibleClassName", "secondaryAction", "selected", "slotProps", "slots"], gt = (t, e) => {
  const { ownerState: s } = t;
  return [e.root, s.dense && e.dense, s.alignItems === "flex-start" && e.alignItemsFlexStart, s.divider && e.divider, !s.disableGutters && e.gutters, !s.disablePadding && e.padding, s.button && e.button, s.hasSecondaryAction && e.secondaryAction];
}, ft = (t) => {
  const { alignItems: e, button: s, classes: a, dense: n, disabled: i, disableGutters: d, disablePadding: p, divider: b, hasSecondaryAction: g, selected: A } = t;
  return D({ root: ["root", n && "dense", !d && "gutters", !p && "padding", b && "divider", i && "disabled", s && "button", e === "flex-start" && "alignItemsFlexStart", g && "secondaryAction", A && "selected"], container: ["container"] }, at, a);
}, vt = N("div", { name: "MuiListItem", slot: "Root", overridesResolver: gt })(({ theme: t, ownerState: e }) => o({ display: "flex", justifyContent: "flex-start", alignItems: "center", position: "relative", textDecoration: "none", width: "100%", boxSizing: "border-box", textAlign: "left" }, !e.disablePadding && o({ paddingTop: 8, paddingBottom: 8 }, e.dense && { paddingTop: 4, paddingBottom: 4 }, !e.disableGutters && { paddingLeft: 16, paddingRight: 16 }, !!e.secondaryAction && { paddingRight: 48 }), !!e.secondaryAction && { [`& > .${rt.root}`]: { paddingRight: 48 } }, { [`&.${m.focusVisible}`]: { backgroundColor: (t.vars || t).palette.action.focus }, [`&.${m.selected}`]: { backgroundColor: t.vars ? `rgba(${t.vars.palette.primary.mainChannel} / ${t.vars.palette.action.selectedOpacity})` : I(t.palette.primary.main, t.palette.action.selectedOpacity), [`&.${m.focusVisible}`]: { backgroundColor: t.vars ? `rgba(${t.vars.palette.primary.mainChannel} / calc(${t.vars.palette.action.selectedOpacity} + ${t.vars.palette.action.focusOpacity}))` : I(t.palette.primary.main, t.palette.action.selectedOpacity + t.palette.action.focusOpacity) } }, [`&.${m.disabled}`]: { opacity: (t.vars || t).palette.action.disabledOpacity } }, e.alignItems === "flex-start" && { alignItems: "flex-start" }, e.divider && { borderBottom: `1px solid ${(t.vars || t).palette.divider}`, backgroundClip: "padding-box" }, e.button && { transition: t.transitions.create("background-color", { duration: t.transitions.duration.shortest }), "&:hover": { textDecoration: "none", backgroundColor: (t.vars || t).palette.action.hover, "@media (hover: none)": { backgroundColor: "transparent" } }, [`&.${m.selected}:hover`]: { backgroundColor: t.vars ? `rgba(${t.vars.palette.primary.mainChannel} / calc(${t.vars.palette.action.selectedOpacity} + ${t.vars.palette.action.hoverOpacity}))` : I(t.palette.primary.main, t.palette.action.selectedOpacity + t.palette.action.hoverOpacity), "@media (hover: none)": { backgroundColor: t.vars ? `rgba(${t.vars.palette.primary.mainChannel} / ${t.vars.palette.action.selectedOpacity})` : I(t.palette.primary.main, t.palette.action.selectedOpacity) } } }, e.hasSecondaryAction && { paddingRight: 48 })), Ct = N("li", { name: "MuiListItem", slot: "Container", overridesResolver: (t, e) => e.container })({ position: "relative" }), yt = l.forwardRef(function(e, s) {
  const a = T({ props: e, name: "MuiListItem" }), { alignItems: n = "center", autoFocus: i = false, button: d = false, children: p, className: b, component: g, components: A = {}, componentsProps: O = {}, ContainerComponent: G = "li", ContainerProps: { className: z } = {}, dense: M = false, disabled: k = false, disableGutters: $ = false, disablePadding: H = false, divider: J = false, focusVisibleClassName: K, secondaryAction: V, selected: Q = false, slotProps: X = {}, slots: Y = {} } = a, q = R(a.ContainerProps, mt), Z = R(a, bt), B = l.useContext(x), S = l.useMemo(() => ({ dense: M || B.dense || false, alignItems: n, disableGutters: $ }), [n, B.dense, M, $]), P = l.useRef(null);
  tt(() => {
    i && P.current && P.current.focus();
  }, [i]);
  const u = l.Children.toArray(p), F = u.length && et(u[u.length - 1], ["ListItemSecondaryAction"]), C = o({}, a, { alignItems: n, autoFocus: i, button: d, dense: S.dense, disabled: k, disableGutters: $, disablePadding: H, divider: J, hasSecondaryAction: F, selected: Q }), U = ft(C), w = st(P, s), y = Y.root || A.Root || vt, f = X.root || O.root || {}, r = o({ className: L(U.root, f.className, b), disabled: k }, Z);
  let c = g || "li";
  return d && (r.component = g || "div", r.focusVisibleClassName = L(m.focusVisible, K), c = ot), F ? (c = !r.component && !g ? "div" : c, G === "li" && (c === "li" ? c = "div" : r.component === "li" && (r.component = "div")), v(x.Provider, { value: S, children: _(Ct, o({ as: G, className: L(U.container, z), ref: w, ownerState: C }, q, { children: [v(y, o({}, f, !j(y) && { as: c, ownerState: o({}, C, f.ownerState) }, r, { children: u })), u.pop()] })) })) : v(x.Provider, { value: S, children: _(y, o({}, f, { as: c, ref: w }, !j(y) && { ownerState: o({}, C, f.ownerState) }, r, { children: [u, V && v(ut, { children: V })] })) });
}), xt = yt;
export {
  xt as L
};
