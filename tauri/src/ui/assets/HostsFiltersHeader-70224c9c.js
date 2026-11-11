/*
 * Recovered Termius UI module: HostsFiltersHeader-70224c9c.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r, bi as W, hE as T, n4 as ie, cl as oe, bq as A, bj as i, wL as ce, cO as le, mL as de, xL as pe, jz as me, Au as ue, cN as Z, gG as he, Av as ve, bX as ge, bt as n, bS as fe, br as p, bu as b, bP as P, pE as xe, iI as U, iJ as V, bO as M, b$ as Se, c0 as Te, c1 as ke, bA as N, Aw as be, f1 as we, f2 as Ie, aH as Pe, cH as ye, Ax as Ee, iF as Ce, Ay as Le, bk as Me, Az as Ne, AA as Oe, AB as We, ew as Ae, AC as Fe, bh as ze, cG as He, iY as Be, gc as De, db as $e, AD as Re, dd as je, de as qe } from "./reconnectSaga-3b11f62e.js";
const _e = (o, s) => r.createElement("svg", { width: 30, height: 30, viewBox: "0 0 30 30", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: s, ...o }, r.createElement("rect", { width: 30, height: 30, rx: 10, transform: "matrix(-1 0 0 1 30 -3.05176e-05)", fill: "#E6EBED", fillOpacity: 0.25 }), r.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M15.0001 13.7143C17.1303 13.7143 18.8572 11.9874 18.8572 9.85714C18.8572 7.7269 17.1303 6 15.0001 6C12.8698 6 11.1429 7.7269 11.1429 9.85714C11.1429 11.9874 12.8698 13.7143 15.0001 13.7143ZM14.9999 24C18.0117 24 20.678 22.5206 22.3117 20.2488C21.2831 17.1974 18.3981 15.0001 14.9998 15.0001C11.6016 15.0001 8.71656 17.1974 7.68799 20.2487C9.32167 22.5205 11.988 24 14.9999 24Z", fill: "#798C94" })), Ge = r.forwardRef(_e), Ue = r.memo(Ge), Ve = ({ className: o, list: s, avatars: c, closeMenu: h, colorScheme: d, fromCollaboration: a, disableScrolling: v = false }) => {
  const { terminalColors: g } = ie(d), { classes: t, cx: k } = Ze({ fromCollaboration: a }), l = r.useRef(oe.v4()), x = A(), y = i(ce), S = i(le), f = i(de), J = i(pe), [X, F] = me(s, ue), Y = Z(), w = i(he), z = i(ve), H = i(ge), E = c.find((e) => e.id === H), [B, D] = r.useState(null), [K, Q] = r.useState(), [m, ee] = r.useState(), $ = r.useCallback((e, u) => {
    D(e), Q(u ?? e.innerText);
  }, []), R = r.useCallback(() => {
    D(null);
  }, []), j = r.useCallback((e) => {
    const u = e.querySelector("div");
    return u ? u.scrollWidth > u.clientWidth : e.scrollWidth > e.clientWidth;
  }, []), C = r.useCallback((e, u) => {
    m && clearTimeout(m);
    const { currentTarget: I } = e;
    j(I) && ee(setTimeout(() => $(I, u), 500));
  }, [$, m, j]), q = r.useCallback(() => {
    R(), m && clearTimeout(m);
  }, [R, m]), L = r.useCallback(() => {
    m && clearTimeout(m);
  }, [m]), te = () => {
    we(Y, { dialog: "TeamTrialViaInvites", target: Ie(), props: { teamTrialFunnelID: l.current }, options: { closeOnBackdropClick: false } }).catch(Pe);
  }, _ = !y && S && !J, ne = () => {
    S || (x(N("teamTrialFunnelStarted", { openedFrom: "Invite People Button", teamTrialFunnelID: l.current, teamTrialPlan: "Team" })), x(N("createTeamWizardViewOpened", { createTeamWizardStep: "Invite Members", openedFrom: "Invite People Button", teamTrialFunnelID: l.current, trial: "yes" }))), _ ? be() : (x(N("inviteScreenOpened", { funnelType: null })), te(), h && h());
  }, G = n(fe, { open: !!B, anchorEl: B, PaperProps: { onMouseLeave: q }, TransitionProps: { timeout: { enter: 200, exit: 0 } }, className: k(t.popover, g), classes: { paper: t.popoverPaper }, elevation: 0, anchorOrigin: { vertical: "center", horizontal: "left" }, transformOrigin: { vertical: "center", horizontal: "left" }, children: K }), se = (
    /* self-cloud:no-team-invite-entry */
    null
  ), re = X.map((e) => {
    const u = e.username === w && e.username;
    return p("div", { className: t.item, children: [n(U, { ...V(e, c, a) }), n("div", { className: t.text, onMouseEnter: (I) => C(I, e.username), onMouseLeave: L, children: n(b, { color: "primary", fontSize: 14, children: a && u ? `${e.userFullName || e.username} (me)` : e.userFullName || e.username }) })] }, `${e.username}-${e.id ?? 0}`);
  }), ae = p(M.Fragment, { children: [n(b, { className: t.pendingInvitesTitle, color: "secondary", fontSize: 12, lineHeight: 1.5, children: "Pending invites:" }), F.map((e) => p("div", { className: t.item, children: [n(U, { ...V(e, c) }), n("div", { className: P(t.text, { [t.textInvited]: !a }), onMouseEnter: C, onMouseLeave: L, children: n(b, { color: "secondary", fontSize: 14, children: e.username }) })] }, `${e.username}-${e.id ?? 0}`))] });
  return p("div", { className: k(t.root, { [t.disableScrolling]: v }, o), onScroll: q, children: [!a && se, S ? p(M.Fragment, { children: [G, re, F.length !== 0 ? ae : null] }) : p(M.Fragment, { children: [G, p("div", { className: t.item, children: [n(Se, { isActive: !a, color: Te(w, H ?? 0), avatar: (E == null ? void 0 : E.base64) || "", letter: ke({ email: w, fullName: z }) }), n("div", { className: t.text, onMouseEnter: C, onMouseLeave: L, children: n(b, { children: z || w }) })] })] })] });
}, Ze = W()((o, { fromCollaboration: s }, c) => ({ root: { minWidth: "243px", maxWidth: "368px", maxHeight: "500px", overflow: "auto", "@media (max-height: 640px)": { maxHeight: "300px" } }, disableScrolling: { overflow: "unset" }, item: { display: "flex", flex: "none", padding: "10px", alignItems: "center", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: "300px" }, text: { marginLeft: "8px", overflow: "hidden", textOverflow: "ellipsis", "& > div": { color: `${s ? T.uiColor : "var(--text-primary)"}`, userSelect: "text" } }, textInvited: { "& > div": { color: `${s ? T.uiColor : "var(--secondary)"}`, userSelect: "text" } }, inviteHover: {}, inviteLink: { display: "flex", alignItems: "center", gap: "10px", cursor: "pointer", margin: "10px", "& rect": { fill: "var(--accent-transparent)" }, "& path": { fill: "var(--accent)" }, "&:hover": { "& svg ": { "& path": { fill: "var( --accent-light)" } } }, [`&:hover .${c.inviteHover}`]: { color: "var(--accent)", fill: "var(--accent)" } }, inviteText: { "&:hover ": { color: "var(--accent)" } }, popover: { pointerEvents: "auto", wordBreak: "break-all" }, popoverPaper: { padding: "5px 8px", borderRadius: "5px", marginLeft: "-5px", filter: "drop-shadow(0px 8px 14px rgba(0, 0, 0, 0.25))", maxWidth: "300px", userSelect: "text", whiteSpace: "normal", color: s ? T.uiColor : void 0, background: s ? T.gradientColor : "var(--white)", ".termius-dark-theme &": { color: s ? T.uiColor : "var(--card-c)", background: s ? T.gradientColor : "var(--white)" } }, linkIcon: { width: "13px", height: "13px", "& path": { fill: "var(--main)", ".termius-dark-theme &": { fill: "var(--main)" } }, alignSelf: "center" }, pendingInvitesTitle: { margin: "10px" } })), Je = ({ team: o, avatars: s, size: c, listTitle: h, onOpenList: d, endPosition: a, onCopyInvitationLink: v, visibleTeamSizeLimit: g }) => {
  const { classes: t } = Xe(), k = a ? { position: "right", anchorOrigin: { vertical: "bottom", horizontal: "center" }, tipMargin: 20 } : {};
  return n("div", { className: t.root, role: "menu", children: n(ye, { className: t.menu, listProps: { disablePadding: true }, ...k, toggler: () => n(Ee, { team: o, avatars: s, onOpenList: d, size: c, visibleTeamSizeLimit: g, mayIncludePlusIcon: true }), children: ({ closeMenu: l }) => n(Ve, { list: o, avatars: s, listTitle: h, onCopyInvitationLink: v, closeMenu: l }) }) });
}, Xe = W()({ root: { margin: "auto" }, menu: { marginTop: "10px", minWidth: "300px" } }), Ye = () => {
  return null;
}, O = "hosts";
function et({ className: o, children: s, onFiltersChanged: c, filters: h }) {
  const { classes: d } = Ke(), a = i(He), v = i(Be), { sizeObserverRef: g, width: t } = De(), l = t - 81, x = A(), y = r.useCallback(({ sort: f }) => {
    f && x($e({ tab: O, sort: f.type, reverse: f.reverse }));
  }, [x]), S = i((f) => f.sorting[O]);
  return p("div", { className: P(o, d.header, { "termius-dark-theme": a }), children: [s, p("div", { className: d.responsivePart, ref: g, children: [l >= 250 && n(Re, { tab: O, dark: a }), l >= 100 && n(je, { dark: a, onChange: c, selectedTags: h.tags }), l >= 200 && n(qe, { type: S.by, reverse: S.reverse, onChange: y, dark: a }), n("div", { children: v && l >= 0 && n(Ye, { endPosition: l < 100 }) })] })] });
}
const Ke = W()({ header: { background: "var(--light-grey-5)", ".termius-dark-theme &": { background: "var(--dark-grey-3)" }, display: "flex", padding: "5px 12px", paddingTop: "4px", height: "45px", alignItems: "center", gap: "15px", "&.termius-dark-theme, .termius-dark-theme &": { borderBottom: "var(--menu-border)", minHeight: "45px" } }, responsivePart: { display: "flex", minWidth: 0, flex: 1, alignItems: "center", gap: "15px", justifyContent: "end" } });
export {
  et as HostsFiltersHeader
};
