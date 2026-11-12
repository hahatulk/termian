/*
 * Recovered Termius UI module: setting-process-8b8e42f8.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as m, rD as _, X as g, rL as w, bq as v, r as o, s6 as L, bt as t, rW as S, rN as R, br as p, bP as x, pE as A, vy as D, vz as O, bg as i, vA as B, sa as n, bj as h, vB as z, vC as N, g4 as V, mS as H, iP as M, sP as U, sQ as W, sC as $, f2 as F, sT as j, sU as q, vp as K, vq as G, f3 as Q, ga as X, vr as Y, vD as J, vt as Z, vs as tt, vu as et, kR as ot, vv as rt, vw as at, vx as st, vE as nt } from "./reconnectSaga-3b11f62e.js";
import { SnackbarController as it } from "./SnackbarController-d2c726d8.js";
import { L as lt } from "./ListItem-f9045f06.js";
import "./ClickAwayListener-fa29c305.js";
const ct = ({ draggable: e = false }) => {
  const s = g().platform === "mac", a = w(), l = v(), r = o.useMemo(() => L(l).filter((c) => c.actionType === "close"), [l]), { cx: d, classes: u } = dt();
  return t(S, { enabled: e, id: "systemActions", className: d(u.root, a), children: !s && t(R, { className: a, actions: r }) });
}, dt = m()({ root: { position: "absolute", left: "0", right: "0", top: "0", display: "flex", flexWrap: "nowrap", padding: 10, justifyContent: "end", [_.systemButtonIconColor.name]: "var(--secondary)", [_.systemButtonIconColorHover.name]: "var(--main)" } }), E = (e) => {
  const { children: s, selected: a, onClick: l, disableRipple: r, link: d, className: u } = e, { classes: c } = pt();
  return p(lt, { onClick: l, selected: a, disableRipple: r, button: true, classes: { root: x(u, { [c.tab]: true, [c.noHover]: !!r }), selected: c.selected }, children: [s, d && t(A, { className: c.linkIcon })] });
};
E.defaultProps = { onClick: () => {
}, selected: false, link: false, disableRipple: false };
const pt = m()((e, s, a) => ({ tab: { "--background-hover-color": "var(--light-grey-6)", "--background-selected-color": "var(--light-grey-5)", "--text-color": "var(--dark-grey-1)", "--link-icon-color": "var(--dark-grey-6)", ".termius-dark-theme &": { "--background-hover-color": "var(--dark-grey-4)", "--background-selected-color": "var(--dark-grey-5)", "--text-color": "var(--white)", "--link-icon-color": "var(--dark-grey-6)" }, border: "none !important", height: "36px", padding: "0 15px", margin: "8px 10px", borderRadius: "10px", width: "auto", color: "var(--text-color)", borderTop: "1px solid var(--dark-grey-1)", fontSize: "14px", lineHeight: "18px", display: "flex", justifyContent: "space-between", cursor: "default", "&:last-child": { borderBottom: "1px solid var(--dark-grey-1)" }, "&:first-of-type": { borderTop: "none" }, "&:hover": { background: "var(--background-hover-color)", color: "var(--text-color)", [`& .${a.linkIcon}`]: { fill: "var(--text-color)" } }, [`&.${a.selected}, &.${a.selected}:hover`]: { color: "var(--text-color)", background: "var(--background-selected-color)" } }, noHover: { "&:hover": { background: "inherit" } }, linkIcon: { width: "13px", height: "13px", marginLeft: "auto", fill: "var(--link-icon-color)", marginRight: "-5px" }, selected: {} })), ut = (e) => {
  const { currentPath: s, tabs: a, navigate: l } = e, { classes: r } = ht();
  return t(D, { className: r.root, component: "nav", children: a.map((d) => t(E, { selected: O(s, d.path.match ?? d.path.value), onClick: () => l(d.path.value), children: d.title }, d.path.value)) });
}, ht = m()({ root: { padding: "0" } }), mt = o.lazy(() => i(() => import("./index-60dc35fe.js"), ["./index-60dc35fe.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./LinkButton-03e176fc.js", "./Done-8ecd181f.js", "./ErrorOutline-9dd3d7a0.js", "./useHandledClick-9dba862b.js", "./SettingsSection-bc44d6bf.js", "./SettingsTabContent-ce30872f.js", "./FiltersButton-2e4a50c6.js", "./UserListItem-e815dc7d.js", "./crownIcon.react-c8ab03e3.js"], import.meta.url)), gt = o.lazy(() => i(() => import("./SSHid-939f9eaa.js"), ["./SSHid-939f9eaa.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./useRequest-2d237c57.js", "./WarnAboutSSHidDeviceCount-09c75630.js", "./sshid-logo-colored.react-d3338d2f.js", "./retry-circular.react-77791386.js", "./openExternal.react-f073251d.js", "./SettingsTabContent-ce30872f.js", "./SSHidLinkWithUsernamePreview-ce35b750.js"], import.meta.url)), vt = o.lazy(() => i(() => import("./index-f96c7727.js"), ["./index-f96c7727.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./SnackbarController-d2c726d8.js", "./ClickAwayListener-fa29c305.js", "./ListItem-f9045f06.js"], import.meta.url)), _t = o.lazy(() => i(() => import("./CreateTeam-32d0381a.js"), ["./CreateTeam-32d0381a.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./LinkButton-03e176fc.js", "./SettingsTabContent-ce30872f.js"], import.meta.url)), ft = o.lazy(() => i(() => import("./index-7ecae23c.js"), ["./index-7ecae23c.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./SettingsSwitch-d35e9651.js", "./FiltersButton-2e4a50c6.js", "./FontAutocomplete-0a9583b4.js", "./SettingsText-0178ea31.js", "./SettingsSection-bc44d6bf.js", "./SettingsTabContent-ce30872f.js"], import.meta.url)), yt = o.lazy(() => i(() => import("./SftpSettings-54c64fdb.js"), ["./SftpSettings-54c64fdb.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./index-bc78ea9d.js", "./SettingsSection-bc44d6bf.js", "./SettingsTabContent-ce30872f.js"], import.meta.url).then((e) => ({ default: e.SftpSettings }))), bt = o.lazy(() => i(() => import("./index-347b6706.js"), ["./index-347b6706.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./SettingsTabContent-ce30872f.js", "./KeyboardKey-4328b011.js"], import.meta.url)), kt = o.lazy(() => i(() => import("./DeveloperTools-e07b2297.js"), ["./DeveloperTools-e07b2297.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./SettingsTabContent-ce30872f.js", "./SettingsSwitch-d35e9651.js", "./SettingsSection-bc44d6bf.js"], import.meta.url)), f = o.lazy(() => i(() => import("./index-382b939e.js"), ["./index-382b939e.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./SettingsTabContent-ce30872f.js"], import.meta.url)), St = o.lazy(() => i(() => import("./QAHelpers-4af305ca.js"), ["./QAHelpers-4af305ca.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./SettingsSection-bc44d6bf.js", "./SettingsTabContent-ce30872f.js"], import.meta.url)), xt = o.lazy(() => i(() => import("./index-919edcdd.js"), ["./index-919edcdd.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./SettingsTabContent-ce30872f.js", "./SettingsSection-bc44d6bf.js", "./AlertDialog-b50b9ed7.js", "./useRequest-2d237c57.js", "./PasswordInput-5b1b9dd4.js", "./PasswordInput-9d57733c.js", "./OTPTokenDialog-33843eb3.js", "./OTPToken-a5994a9f.js", "./SettingsText-0178ea31.js", "./mailto-0aef9dd0.js", "./SettingsSwitch-d35e9651.js", "./triggerChoosePlanDialog-b93628a0.js", "./upgradeIconGradient.react-fa42469b.js", "./LinkButton-03e176fc.js"], import.meta.url).then(({ Profile: e }) => ({ default: e }))), Et = o.lazy(() => i(() => import("./Vaults-d565f151.js"), ["./Vaults-d565f151.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./PageWrapper-a7d0cf6b.js", "./vaultStatsIconsMap-a07c2efa.js", "./UserListItem-e815dc7d.js", "./crownIcon.react-c8ab03e3.js", "./SelectMemberRole-bca8115e.js", "./Divider-8ca798ff.js", "./checkIconGradient.react-d8f1a390.js", "./ClickAwayListener-fa29c305.js"], import.meta.url).then(({ Vaults: e }) => ({ default: e }))), Pt = o.lazy(() => i(() => import("./LogsSettings-c332b503.js"), ["./LogsSettings-c332b503.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./PageWrapper-a7d0cf6b.js", "./PromoBanner-aac31425.js", "./triggerChoosePlanDialog-b93628a0.js"], import.meta.url).then(({ LogsSettings: e }) => ({ default: e }))), Tt = (e) => {
  const { opacity: s = true, onTransitionEndContainer: a, containerRef: l } = e, { classes: r } = Ct(), { tabs: d, currentPath: u, navigate: c } = It(), T = o.useCallback(() => {
    c("/productboard/main");
  }, [c]), C = o.useCallback((I) => {
    c(`/productboard/${I}`);
  }, [c]);
  return p("div", { className: x({ [r.root]: true, [r.transitionContainer]: true, [r[s ? "opacityOn" : "opacityOff"]]: true }), onTransitionEnd: a, ref: l, children: [t(S, { className: r.settingsHeader, children: "Settings" }), p("div", { className: r.settingsContent, children: [p("div", { className: r.sidebar, children: [t("div", { className: r.tabs, children: t(ut, { currentPath: u, navigate: c, tabs: d }) }), null] }), t(o.Suspense, { fallback: "", children: p("div", { className: r.tabContent, children: [
    /* termian:settings-routes */
    t(n, { path: "/account", children: t(xt, {}) }),
    t(n, { path: "/terminal", children: t(ft, { onOpenProductBoard: C }) }),
    t(n, { path: "/sftp", children: t(yt, {}) }),
    t(n, { path: "/logs", children: t(Pt, {}) }),
    t(n, { path: "/shortcuts", children: t(bt, {}) }),
    t(n, { path: "/version", children: t(vt, {}) })
  ] }) })] }), t(it, {})] });
}, Ct = m()({ root: { "--header-height": "80px", "--border-color": "var(--light-grey-5)", "--text-color": "var(--dark-grey-1)", "--close-button-color": "var(--light-grey-5)", ".termius-dark-theme &": { "--border-color": "var(--dark-grey-1)", "--text-color": "white", "--close-button-color": "var(--dark-grey-5)" }, position: "absolute", top: "0", bottom: "0", left: "0", right: "0", zIndex: 50, opacity: "1", background: "var(--foreground)" }, settingsHeader: { position: "relative", background: "var(--foreground)", borderBottom: "1px solid var(--border-color)", color: "var(--text-color)", fontWeight: 700, padding: "25px 20px 0 20px", height: "var(--header-height)", alignItems: "center", lineHeight: "45px", zIndex: 1 }, sidebar: { position: "relative", width: "220px", height: "100%", background: "var(--foreground)", borderRight: "1px solid var(--border-color)", display: "flex", flexDirection: "column", justifyContent: "space-between", overflowY: "auto" }, tabs: { width: "100%" }, settingsContent: { position: "relative", height: "calc(100% - var(--header-height))" }, tabContent: { height: "100%", position: "absolute", left: "220px", top: "0", right: "0", zIndex: 2 }, ...B, transitionContainer: { transition: "all 0.20s" } });
function It() {
  const e = h(z), s = v(), a = o.useCallback((r) => {
    s(N(r));
  }, [s]);
  V(() => {
    a(e.defaultPath);
  });
  const l = h(H);
  return { ...e, currentPath: l, navigate: a };
}
const wt = () => {
  const e = v(), { classes: s } = Lt(), a = h(M), l = h(U), r = h(W) || l;
  return o.useEffect(() => {
    e($({ title: "Settings", windowId: F() }));
  }, [e]), p("div", { className: s.root, children: [t(j, {}), t(q, { show: r }), t("div", { className: s.root, children: a && t(Tt, { tab: 0 }) })] });
}, Lt = m()({ root: { width: "100%", height: "100%" } });
K();
G();
Q.configure({ stopEventPropagationAfterHandling: false, stopEventPropagationAfterIgnoring: false, ignoreRepeatedEventsWhenKeyHeldDown: false });
document.addEventListener("keydown", (e) => {
  e.code === "KeyW" && e.metaKey && g().platform === "mac" && (e.preventDefault(), X().close());
});
const { store: Rt, sagas: P } = Y();
P.run(J);
const y = document.getElementById("root");
var b, k;
if (y != null) {
  (b = document.body) == null || b.classList.add(g().platform), (k = document.body) == null || k.classList.add("settings");
  const e = Z(y);
  tt(), e.render(t(et, { value: ot, children: t(rt, { value: at, children: p(st, { sagas: P, store: Rt, children: [t(wt, {}), t(ct, {}), g().platform === "mac" && t(nt, {})] }) }) }));
}
export {
  E as S
};
