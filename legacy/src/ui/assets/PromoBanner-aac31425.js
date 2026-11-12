/*
 * Recovered Termius UI module: PromoBanner-aac31425.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as Z, bj as u, zx as H, zy as v, bZ as V, bq as k, cN as Y, vR as z, r as I, gq as G, bt as c, br as h, bs as N, bu as S, cv as y, zz as F, gr as $, zA as j, cT as q, gt as m, vT as W, f1 as A, f2 as B } from "./reconnectSaga-3b11f62e.js";
import { t as K } from "./triggerChoosePlanDialog-b93628a0.js";
const n = { ENABLE_SYNC: "enable_session_logs_sync", ENABLE_SHARING: "enable_session_logs_sharing", ENABLE_RETENTION: "enable_session_logs_retention" }, s = { ENABLE_SYNC_OVER_SIZE_LIMIT: "enable_session_logs_sync_over_size_limit", UNAUTHORIZED_SETUP_VAULT: "unauthorized_setup_vault", UNAUTHORIZED_OVER_SIZE_LIMIT: "unauthorized_over_size_limit" }, X = [n.ENABLE_SYNC, n.ENABLE_SHARING, n.ENABLE_RETENTION, s.ENABLE_SYNC_OVER_SIZE_LIMIT, s.UNAUTHORIZED_SETUP_VAULT, s.UNAUTHORIZED_OVER_SIZE_LIMIT], J = new Set(X), re = (e) => e != null && J.has(e), Q = (e) => {
  if (!e) return "Store More Logs";
  switch (e) {
    case n.ENABLE_SYNC:
      return "Keep Logs In Sync With Pro";
    case n.ENABLE_SHARING:
      return "Troubleshooting In Team with Logs";
    case n.ENABLE_RETENTION:
      return "Audit-ready with logs";
    case s.ENABLE_SYNC_OVER_SIZE_LIMIT:
      return "Store More Logs";
    case s.UNAUTHORIZED_SETUP_VAULT:
      return "Back Up Logs With Account";
    case s.UNAUTHORIZED_OVER_SIZE_LIMIT:
    default:
      return "Store More Logs";
  }
}, ce = () => {
  const { classes: e } = ee(), o = k(), i = Y(), a = se(), _ = u(v), [r] = u(z), [l, g] = I.useState(false), T = u((E) => E.ui.isSessionLogsBannerDismissed), p = a ? T.includes(a) : false, t = Q(a), R = () => {
    o(m({ promotionLocation: "Logs", promotionType: t, trialDaysLeft: r, sideMenuTeamTrialPromoType: void 0 })), o(W("/logs"));
  }, b = async () => {
    o(m({ promotionLocation: "Logs", promotionType: t, trialDaysLeft: r, sideMenuTeamTrialPromoType: void 0 })), await A(i, { dialog: "InviteMembersTrialOnboarding", target: B(), props: {} });
  }, O = async () => {
    o(m({ promotionLocation: "Logs", promotionType: t, trialDaysLeft: r, sideMenuTeamTrialPromoType: void 0 })), A(i, { dialog: "CreateAccountDialog", target: B(), options: { closeOnBackdropClick: false }, props: {} });
  }, f = () => {
    o(m({ promotionLocation: "Logs", promotionType: t, trialDaysLeft: r, sideMenuTeamTrialPromoType: void 0 })), K({ sagas: i, funnelIdValue: "logs-promo", isTriggeredByUser: true });
  }, M = () => {
    a && (o($({ promotionLocation: "Logs", promotionType: t, trialDaysLeft: r, sideMenuTeamTrialPromoType: void 0, dismissForever: void 0 })), o(j(a)));
  }, x = () => g(true), P = () => g(false), d = ne(a, _.size ?? 0, f, R, b, O), w = a !== s.ENABLE_SYNC_OVER_SIZE_LIMIT, L = I.useRef(false);
  return I.useEffect(() => {
    d && !p && !L.current && (L.current = true, o(G({ promotionLocation: "Logs", promotionType: t, trialDaysLeft: r, sideMenuTeamTrialPromoType: void 0 })));
  }, [d, p, o, t, r]), !d || p ? null : c("div", { onMouseEnter: x, onMouseLeave: P, className: e.root, children: h(N, { direction: "row", justifyContent: "space-between", gap: "10px", alignItems: "center", className: e.content, children: [h(N, { direction: "row", gap: "10px", alignItems: "center", children: [h(S, { color: "main", variant: "body1", children: [c(S, { component: "span", bold: true, display: "inline", children: d.title }), " ", d.description] }), c(N, { direction: "row", gap: "10px", alignItems: "center", flexShrink: 0, children: d.actions.map(({ text: E, onClick: D, rightIcon: C }) => c(y, { size: "small", color: "regularLight", onClick: D, rightIcon: C, className: e.button, children: c(S, { variant: "body2", children: E }) }, E)) })] }), w && c(y, { size: "small", variant: "icon", disableHover: true, disableRipple: true, title: "Dismiss banner", onClick: M, className: e.dismissButton, children: c(F, { className: `${e.dismissButtonIcon} ${l ? e.dismissButtonIconHovered : ""}` }) })] }) });
}, ee = Z()({ root: { borderRadius: "15px", border: "1px solid #E6EBED", background: "linear-gradient(111deg, #F7F9FA 0.18%, rgba(247, 249, 250, 0.00) 100.18%)", minHeight: "54px", display: "flex", alignItems: "center", ".termius-dark-theme &": { border: "1px solid #282B3D", background: "linear-gradient(91deg, #282B3D 0.31%, rgba(40, 43, 61, 0.00) 99.62%)" } }, content: { padding: "10px 10px 10px 20px", width: "100%" }, button: { textTransform: "none" }, dismissButton: { position: "relative" }, dismissButtonIcon: { width: "14px", height: "14px", transition: "color 0.2s ease-in-out, opacity 0.2s ease-in-out", color: "var(--light-grey-4)", opacity: 0, "&:hover": { color: "var(--light-grey-3)" }, ".termius-dark-theme &": { color: "var(--dark-grey-5)", "&:hover": { color: "var(--dark-grey-6)" } } }, dismissButtonIconHovered: { opacity: 1 } }), oe = (e, o, i) => i ? o ? s.UNAUTHORIZED_OVER_SIZE_LIMIT : s.UNAUTHORIZED_SETUP_VAULT : e === n.ENABLE_SYNC && o ? s.ENABLE_SYNC_OVER_SIZE_LIMIT : e, se = () => {
  const e = u(H), o = u(v), i = u(V);
  return oe(e, o.over, i);
}, te = c(q, { width: "14px", height: "14px" }), U = { [n.ENABLE_SYNC]: { title: "", description: "Keep your logs in sync across mobile and desktop with cloud vault.", actions: [{ text: "Sync now", actionType: "upgrade", showUpgradeIcon: true }] }, [n.ENABLE_SHARING]: { title: "", description: "Troubleshoot faster with your teammates by accessing shared logs in Team vault.", actions: [{ text: "Invite members", actionType: "invite" }] }, [n.ENABLE_RETENTION]: { title: "", description: "Stay audit-ready with a custom log retention policy that fits your compliance requirements.", actions: [{ text: "Upgrade now", actionType: "upgrade", showUpgradeIcon: true }] }, [s.ENABLE_SYNC_OVER_SIZE_LIMIT]: { title: (e) => `You have ${e}mb of logs stored locally.`, description: "Manage log storage in settings.", actions: [{ text: "Manage", actionType: "manage" }] }, [s.UNAUTHORIZED_SETUP_VAULT]: { title: "", description: "Back up your logs with cloud vault and access them on mobile and desktop.", actions: [{ text: "Set up vault", actionType: "setup" }] }, [s.UNAUTHORIZED_OVER_SIZE_LIMIT]: { title: (e) => `You have ${e}mb of logs stored locally.`, description: "Change log retention policies or create an account to store logs in a cloud vault.", actions: [{ text: "Manage", actionType: "manage" }, { text: "Create account", actionType: "setup" }] } }, ne = (e, o, i, a, _, r) => {
  if (!e || !U[e]) return null;
  const l = U[e], g = typeof l.title == "function" ? l.title(o) : l.title, T = { upgrade: i, manage: a, invite: _, setup: r }, p = l.actions.map((t) => ({ text: t.text, onClick: T[t.actionType], rightIcon: t.showUpgradeIcon ? te : void 0 }));
  return { title: g, description: l.description, actions: p };
};
export {
  s as E,
  ce as P,
  re as s,
  se as u
};
