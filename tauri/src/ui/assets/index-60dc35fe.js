/*
 * Recovered Termius UI module: index-60dc35fe.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { ds as O, dt as D, du as H, bi as v, bt as e, fu as F, br as r, dG as A, bu as u, dE as j, bP as B, cN as k, r as b, ew as y, Bk as W, bs as m, bz as R, cH as z, cY as q, cL as U, aH as V, e7 as X, M as Y, rC as G, pm as J, Bl as K, yd as Q, bj as g, wm as Z, Bm as ee, Bn as te, bk as se, ca as ae, bw as L, Bo as ne, uX as re, yk as ie, bq as oe, Ay as ce, xL as le, fH as de, Bp as ue, Bq as me, Aw as pe, AB as he, AC as ge, bh as fe } from "./reconnectSaga-3b11f62e.js";
import { L as ve } from "./LinkButton-03e176fc.js";
import { d as be } from "./Done-8ecd181f.js";
import { d as ye } from "./ErrorOutline-9dd3d7a0.js";
import { u as xe } from "./useHandledClick-9dba862b.js";
import { S as T, a as Se } from "./SettingsSection-bc44d6bf.js";
import { S as ke } from "./SettingsTabContent-ce30872f.js";
import { F as Ie } from "./FiltersButton-2e4a50c6.js";
import { U as N } from "./UserListItem-e815dc7d.js";
import { M as we } from "./crownIcon.react-c8ab03e3.js";
var w = {}, Ce = D;
Object.defineProperty(w, "__esModule", { value: true });
var E = w.default = void 0, _e = Ce(O()), Le = H, Te = (0, _e.default)((0, Le.jsx)("path", { d: "M3.9 12c0-1.71 1.39-3.1 3.1-3.1h4V7H7c-2.76 0-5 2.24-5 5s2.24 5 5 5h4v-1.9H7c-1.71 0-3.1-1.39-3.1-3.1zM8 13h8v-2H8v2zm9-6h-4v1.9h4c1.71 0 3.1 1.39 3.1 3.1s-1.39 3.1-3.1 3.1h-4V17h4c2.76 0 5-2.24 5-5s-2.24-5-5-5z" }), "Link");
E = w.default = Te;
const Ne = { exit: 500 }, x = ({ isVisible: t, children: s }) => {
  const { classes: n } = Me();
  return e(F, { in: t, timeout: Ne, children: e("div", { className: n.item, children: s }) });
}, Me = v()({ item: { position: "absolute", display: "flex", alignItems: "center", left: 0 } }), Ae = "Copy invitation link", Be = "Copy invitation link", Re = "Invitation link copied!", Ee = "Something went wrong!";
function Pe({ className: t, onClick: s, status: n }) {
  const { classes: a, cx: i } = $e();
  return r("div", { className: i(a.inviteLink, t), children: [r(x, { isVisible: n === "default", children: [e(E, { className: i(a.link, a.icon) }), e(A, { onClick: s, onContextMenu: s, children: e(u, { color: "inherit", children: Ae }) })] }), r(x, { isVisible: n === "success", children: [e(be, { className: a.icon }), e(u, { color: "inherit", children: Re })] }), r(x, { isVisible: n === "error", children: [e(ye, { className: a.icon }), e(u, { color: "inherit", children: Ee })] }), r(x, { isVisible: n === "loading", children: [e(j, { className: a.icon, size: 24, classes: { colorPrimary: a.progressColor } }), e(u, { color: "inherit", children: Be })] })] });
}
const $e = v()({ inviteLink: { fontSize: "14px", lineHeight: "14px", textTransform: "initial", padding: 0, fontWeight: 500, position: "relative", width: "200px", height: "24px", display: "flex", color: "var(--accent)" }, link: { transform: "rotate(-60deg)" }, icon: { marginRight: "5px", color: "var(--light-grey-1)", ".termius-dark-theme &": { color: "var(--white)" } }, progressColor: { color: "var(--light-grey-1)", ".termius-dark-theme &": { color: "var(--white)" } } }), Oe = ({ className: t, onClick: s }) => {
  const { classes: n } = De(), { handleClick: a, status: i } = xe({ onClick: s });
  return e(Pe, { className: B(t, { [n.success]: i === "success" }), status: i, onClick: a });
}, De = v()({ success: { color: "var(--dark-grey-1)", ".termius-dark-theme &": { color: "var(--white)" } } });
var P = ((t) => (t.enabled = "Enabled", t.disabled = "Disabled", t))(P || {});
const S = Object.values(P), M = ({ isSelected: t, name: s, children: n }) => {
  const a = k(), [i, p] = b.useState(false), { classes: o } = He(), h = b.useCallback(async (l) => {
    p(true);
    try {
      await y(a, W, { [s]: l });
    } finally {
      p(false);
    }
  }, [a, s]);
  return r(m, { className: o.securityRowContainer, children: [e(m, { flexDirection: "row", gap: "10px", alignItems: "center", children: n }), i && e(m, { ml: "auto", children: e(R, { size: 15, className: o.spinner }) }), e(z, { toggler: (l) => r(Ie, { className: o.text, children: [t ? "Enabled" : "Disabled", e(q, { className: B(o.arrowIcon, { [o.open]: l.isOpen }) })] }), children: ({ closeMenu: l }) => S.map((d) => e(U, { dense: true, onClick: () => {
    h(d === S[0]).catch(V), l();
  }, endAdornment: t && d === S[0] || !t && d === S[1] ? e(X, {}) : void 0, children: d }, d)) })] });
}, He = v()({ securityRowContainer: { flexDirection: "row", justifyContent: "space-between", alignItems: "center", width: "100%", marginTop: "15px" }, arrowIcon: { width: "15px", height: "6px", paddingLeft: "5px", fill: "var(--filters-header-color)", marginTop: "2px" }, open: { transform: "rotate(180deg)", paddingLeft: "0px", paddingRight: "5px" }, text: { color: "var(--secondary)", fontWeight: 450 }, spinner: { color: "var(--secondary)" } }), f = { active: "Active", pendingAccess: "Pending access", error: "Send failed", resendInvite: "Resend invite", sending: "Sending...", invitationSent: "Invitation sent" }, Fe = ({ id: t }) => {
  const { classes: s, cx: n } = je(), a = k(), [i, p] = b.useState(f.resendInvite), o = b.useRef(), [h, l] = b.useState(true);
  Y(() => {
    o.current && clearTimeout(o.current);
  });
  const [d] = G(() => ({ from: { opacity: h ? 1 : 0 }, to: { opacity: 1 }, onResolve: () => {
    l(false);
  }, reset: !h }), [i]);
  return e(A, { onClick: async () => {
    p(f.sending);
    try {
      await y(a, K, { id: t }), p(f.invitationSent);
    } catch {
      p(f.error);
    }
    o.current = setTimeout(() => p(f.resendInvite), 2e3);
  }, className: n(s.status, s.resendButton, { [s.resendError]: i === f.error }), color: "accent", disabled: i !== f.resendInvite, children: e(J.div, { style: d, children: i }) });
}, je = v()({ status: { display: "flex", flexShrink: 0, minWidth: "108px", marginLeft: "auto", paddingLeft: "10px" }, resendButton: { lineHeight: 1.5, "&:disabled": { color: "var(--accent)" } }, resendError: { "&:disabled": { color: "var(--error-medium)" } } }), We = () => {
  const { classes: t } = $(), s = k(), [n, a] = b.useState(false), i = Q(), p = g(Z), o = g(ee), h = g(te), l = g(se), d = h.account.user_id, C = async () => {
    try {
      a(true), await i(Promise.all([y(s, ne), y(s, re)])), a(false);
    } catch (c) {
      c instanceof ie || a(false);
    } finally {
      a(false);
    }
  };
  ae(() => {
    C();
  });
  const _ = qe({ members: p, owner: h.account });
  return r(m, { children: [r(m, { className: t.listHeader, children: [r(m, { flexDirection: "row", alignItems: "center", gap: "12px", children: [e(u, { variant: "subtitle1", color: "main", children: "Member" }), n && e(R, { size: 15, color: "inherit" })] }), e(u, { variant: "subtitle1", color: "main", className: t.status, children: "Status" })] }), r(m, { component: "ul", className: t.membersList, children: [_.map((c) => e(N, { user: c, avatar: L(l, c.user_id), disabled: true, inlineIcons: e(ze, { member: c, ownerId: d, currentUserId: d }), titleClassName: I(c) ? "" : t.secondaryText, endAdornment: e(u, { color: I(c) ? "primary" : "secondary", className: t.status, children: I(c) ? "Active" : "Pending access" }) }, c.user_id)), o.length > 0 && o.map((c) => e(N, { avatar: L(l, c.user_id), user: c, disabled: true, className: t.inviteListItem, titleClassName: t.secondaryText, endAdornment: e(Fe, { id: c.id }) }, c.id))] })] });
}, ze = ({ member: t, ownerId: s, currentUserId: n }) => {
  const { classes: a } = $();
  return r(m, { className: a.inlineBadges, children: [s === t.user_id && e(m, { className: a.ownerAdditionalSectionWrapper, children: e(we, {}) }, "owner"), n === t.user_id && e(m, { className: a.ownerAdditionalSectionWrapper, children: e(u, { fontSize: 7, bolder: true, className: a.ownerAdditionalSectionLabel, display: "inline", children: "YOU" }) }, "you")] });
};
function qe({ members: t, owner: s }) {
  return [{ id: s.user_id ?? -1, email: s.email ?? "", full_name: s.full_name, user_id: s.user_id ?? -1, is_access_granted: true, latest_activity: "", two_factor_auth: s.two_factor_auth, has_key_pair: true, role: "Owner" }, ...t];
}
function I(t) {
  return "is_access_granted" in t ? !!t.is_access_granted : false;
}
const $ = v()({ listHeader: { flexDirection: "row", alignItems: "center", padding: "10px 15px", borderRadius: "10px", justifyContent: "space-between", backgroundColor: "var(--card-b)", ".termius-dark-theme &": { backgroundColor: "var(--card-c)" } }, ownerAdditionalSectionWrapper: { justifyContent: "center", alignItems: "center", width: "25px", height: "15px", background: "var(--gradient-light-main)", borderRadius: "4px", ".termius-dark-theme &": { background: "var(--main-gradient-outline)" } }, ownerAdditionalSectionLabel: { lineHeight: 1, background: "var(--gradient-universal-main)", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent", textFillColor: "transparent" }, status: { display: "flex", flexShrink: 0, minWidth: "108px", marginLeft: "auto", paddingLeft: "10px" }, inlineBadges: { flexDirection: "row", alignItems: "center", marginLeft: "10px", gap: "5px" }, membersList: { overflow: "hidden overlay", "&::-webkit-scrollbar": { display: "none" } }, secondaryText: { color: "var(--secondary)" }, inviteListItem: { pointerEvents: "initial" } }), Ue = () => {
  const { classes: t } = Ve(), s = oe(), n = k(), a = g(ce), i = g(le), p = g(de), o = g(ue) || false, h = g(me) || false, l = async () => {
    s(he({ inviteLinkLocation: "Team Screen" }));
    const d = await y(n, ge, { createTeam: false });
    fe.writeText(d.url);
  };
  return r(ke, { children: [r(T, { className: t.teamContainer, children: [r(m, { flexDirection: "row", justifyContent: "space-between", children: [e(u, { variant: "subtitle1", color: "main", ellipsis: true, bolder: true, children: p }), e(ve, { onClick: pe, variant: "text", color: "accent", disableHover: true, className: t.manageButton, children: "Manage" })] }), e(We, {}), a && e(Oe, { className: t.copyLinkButton, onClick: l })] }), !i && r(T, { children: [e(Se, { children: "Security" }), r(m, { children: [r(M, { isSelected: o, name: "realtime_collaboration", children: [e(u, { color: "main", children: "Multiplayer" }), e(u, { color: "secondary", children: "Beta" })] }), e(M, { isSelected: h, name: "two_factor_auth", children: e(u, { color: "main", children: "Require 2FA for all team members" }) })] })] })] });
}, Ve = v()({ teamContainer: { display: "flex", flexDirection: "column", gap: 20 }, copyLinkButton: { color: "--var(--accent)", "& svg": { fill: "var(--accent)" } }, manageButton: { padding: 0, height: "auto", "&:hover:not(:focus-visible):not(:disabled)": { color: "var(--accent-light)" }, "&:focus-visible": { background: "var(--accent-transparent)" } } }), at = Ue;
export {
  at as default
};
