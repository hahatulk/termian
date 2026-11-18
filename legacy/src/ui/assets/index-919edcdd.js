/*
 * Recovered Termius UI module: index-919edcdd.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
var Ne = Object.defineProperty;
var Re = (t, s, r) => s in t ? Ne(t, s, { enumerable: true, configurable: true, writable: true, value: r }) : t[s] = r;
var A = (t, s, r) => (Re(t, typeof s != "symbol" ? s + "" : s, r), r);
import { dZ as I, cu as U, cN as P, bq as E, bO as T, ft as He, ya as Ie, wa as K, ew as F, Cr as Ve, Cs as $e, xi as X, br as a, cD as _, xj as D, bt as e, yb as Oe, r as n, gv as We, Ct as Ze, gf as ze, bi as m, bj as i, cG as we, bu as v, cy as q, Cu as Ue, yz as _e, Cv as qe, dl as je, cs as Ge, yE as Ye, f1 as O, f2 as W, aH as Z, Cw as Ke, Cx as Xe, kQ as ue, cv as H, yH as Je, bR as N, mL as J, pE as Qe, bP as z, Cy as et, gP as fe, Cz as Q, CA as tt, CB as st, CC as ee, d4 as te, bV as rt, Ai as nt, CD as ot, CE as at, CF as it, bo as ct, gO as se, wm as be, Aj as lt, wN as dt, CG as ut, CH as ht, CI as mt, xb as he, gp as pt, wu as ye, xL as gt, xs as Ct, X as Y, CJ as wt, dS as me, dT as $, bs as pe, Aw as ft, wp as Se, CK as bt, CL as yt, CM as St, ud as ge, CN as xt, CO as vt, l5 as Tt, vR as Pt, CP as Et, wL as Lt, gG as Dt, xa as kt, wS as Mt, wv as At, CQ as Bt, CR as Ft, x1 as Nt, CS as Rt, bg as Ht, yL as It, az as Vt, Bn as $t, cO as Ot } from "./reconnectSaga-3b11f62e.js";
import { S as Wt } from "./SettingsTabContent-ce30872f.js";
import { a as R, S as B } from "./SettingsSection-bc44d6bf.js";
import { r as re } from "./AlertDialog-b50b9ed7.js";
import { u as Zt } from "./useRequest-2d237c57.js";
import { F as Ce } from "./PasswordInput-5b1b9dd4.js";
import { r as ne } from "./OTPTokenDialog-33843eb3.js";
import { P as zt } from "./PasswordInput-9d57733c.js";
import { S as k } from "./SettingsText-0178ea31.js";
import { u as Ut } from "./mailto-0aef9dd0.js";
import { S as xe } from "./SettingsSwitch-d35e9651.js";
import { t as oe } from "./triggerChoosePlanDialog-b93628a0.js";
import { M as _t } from "./upgradeIconGradient.react-fa42469b.js";
import { L as ve } from "./LinkButton-03e176fc.js";
import "./OTPToken-a5994a9f.js";
function qt(t) {
  const { onClose: s } = t, r = P(), o = E(), [c, l] = T.useState(), { register: d, formState: u, watch: C } = He({ mode: "onChange", defaultValues: { oldPassword: "", newPassword: "" } }), p = C(), { isPasswordValidForSignUp: b, ...w } = Ie(p.newPassword, { prevPassword: p.oldPassword }), y = Zt(async (h) => {
    const { oldPassword: f, newPassword: g } = p;
    return F(r, Ve, { oldPassword: f, newPassword: g, authyToken: h });
  }, { manual: true, onSuccess: () => {
    s(), re({ title: "Success", text: `${K(D)} has been changed successfully` });
  }, onError: (h, [f]) => L(h, f) }), L = (h, f) => {
    if (!(h instanceof $e)) {
      l(h.message);
      return;
    }
    h.status === 487 && !f ? (s(), o(X()), ne({ submitCallback: y.runAsync })) : l(h.message);
  }, x = () => {
    y.run();
  };
  return a(_, { title: `Change ${D}`, buttonColor: "accent", confirmButtonTitle: "Save", fetching: y.loading, disabled: !b, onOk: x, onClose: s, submitErrorText: c, noAutoFocus: true, children: [e(Ce, { autoFocus: true, fullWidth: true, label: `Old ${D}`, error: u.errors.oldPassword !== void 0, InputProps: { ...d("oldPassword", { required: true }) } }), e(Ce, { fullWidth: true, label: `New ${D}`, error: u.errors.newPassword !== void 0, InputProps: { ...d("newPassword", { required: true }) } }), e(Oe, { ...w })] });
}
const jt = I.promisify(U(qt));
class Te extends n.Component {
  constructor() {
    super(...arguments);
    A(this, "state", { fetching: false, username: "", password: "", submitErrorText: "", errors: { username: false, password: false } });
    A(this, "onChange", (r) => (o) => this.setState({ [r]: o.target.value, submitErrorText: "", errors: { ...this.state.errors, [r]: !o.target.value } }));
    A(this, "onSubmit", async (r) => (r.preventDefault(), this.changeUsername()));
    A(this, "changeUsername", async (r) => {
      this.setState({ fetching: true });
      const { username: o, password: c } = this.state;
      try {
        await F(this.context, Ze, { username: o, password: c, authyToken: r }), this.props.onClose(), re({ title: "Success", text: "Username has been changed successfully" });
      } catch (l) {
        if (r) throw l;
        if (l.status === 487 && !r) {
          this.props.onClose();
          try {
            this.props.dispatch(X()), await ne({ submitCallback: this.changeUsername });
          } catch {
          }
        } else this.setState({ fetching: false, submitErrorText: l.message });
      }
    });
    A(this, "isDisabled", () => this.state.errors.password || this.state.errors.username || this.state.fetching);
  }
  render() {
    return a(_, { title: "Change email", buttonColor: "accent", confirmButtonTitle: "Save", fetching: this.state.fetching, disabled: this.isDisabled(), onOk: this.onSubmit, onClose: this.props.onClose, submitErrorText: this.state.submitErrorText, noAutoFocus: true, children: [e(ze, { label: "New email", value: this.state.username, onChange: this.onChange("username"), onBlur: this.onChange("username"), error: this.state.errors.username, required: true, fullWidth: true, autoFocus: true }), e(zt, { label: K(D), value: this.state.password, onChange: this.onChange("password"), onBlur: this.onChange("password"), error: this.state.errors.password, type: "password", required: true, fullWidth: true })] });
  }
}
A(Te, "contextType", We);
const Gt = I.promisify(U(Te)), Yt = (t, s) => n.createElement("svg", { width: 269, height: 70, viewBox: "0 0 269 70", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: s, ...t }, n.createElement("rect", { x: 42.1716, y: 22.8846, width: 64.1744, height: 5.38461, rx: 2.69231, fill: "#2091F6" }), n.createElement("rect", { x: 71.3462, width: 126.538, height: 70, rx: 2, fill: "#A4B3BA" }), n.createElement("rect", { x: 133.269, width: 64.6154, height: 70, rx: 2, fill: "#D5DDE0" }), n.createElement("path", { d: "M74.0386 0H193.846L133.942 40.3846L74.0386 0Z", fill: "#EDF1F2" }), n.createElement("rect", { x: 22.0027, y: 43.0769, width: 64.1744, height: 5.38461, rx: 2.69231, fill: "#2091F6" }), n.createElement("rect", { y: 32.3077, width: 64.1744, height: 5.38461, rx: 2.69231, fill: "#2091F6" })), Kt = n.forwardRef(Yt), Xt = n.memo(Kt), Jt = (t, s) => n.createElement("svg", { width: 269, height: 70, viewBox: "0 0 269 70", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: s, ...t }, n.createElement("rect", { x: 42.1716, y: 22.8846, width: 64.1744, height: 5.38461, rx: 2.69231, fill: "#21B568" }), n.createElement("rect", { x: 71.3462, width: 126.538, height: 70, rx: 2, fill: "#3E4257" }), n.createElement("rect", { x: 133.269, width: 64.6154, height: 70, rx: 2, fill: "#5A5E73" }), n.createElement("path", { d: "M74.0386 0H193.846L133.942 40.3846L74.0386 0Z", fill: "#8D91A5" }), n.createElement("rect", { x: 22.0027, y: 43.0769, width: 64.1744, height: 5.38461, rx: 2.69231, fill: "#21B568" }), n.createElement("rect", { y: 32.3077, width: 64.1744, height: 5.38461, rx: 2.69231, fill: "#21B568" })), Qt = n.forwardRef(Jt), es = n.memo(Qt), ts = (t) => {
  const { onClose: s, onOk: r } = t, { classes: o } = ss(), c = i(we);
  return a(_, { title: "Check you inbox", onClose: s, onOk: r, confirmButtonTitle: "Done", buttonColor: "regular", children: [e(v, { color: "primary", children: "We’ve sent you an email with a link to complete the account deletion." }), e(q, { top: 30, bottom: 30, children: e("div", { className: o.image, children: c ? e(es, {}) : e(Xt, {}) }) })] });
}, ss = m()({ image: { display: "flex", justifyContent: "center", "& svg": { width: "269px", height: "70px" } } }), rs = I.promisify(U(ts)), ns = (t) => {
  const { onClose: s } = t, r = P(), o = E(), { classes: c } = os(), [l, d] = n.useState(false), [u, C] = n.useState(""), [p, b] = n.useState(""), [w, y] = n.useState({ password: false }), L = (g) => {
    C(g.target.value), y({ password: !g.target.value });
  }, x = async (g) => (g.preventDefault(), h()), h = async (g) => {
    d(true);
    try {
      await F(r, qe, { password: u, authyToken: g }), s(), rs();
    } catch (S) {
      if (g) throw S;
      if (S.status === 487 && !g) {
        s();
        try {
          o(X()), await ne({ submitCallback: h });
        } catch {
        }
      } else d(false), b(S.message);
    }
  }, f = () => w.password || l || u.length === 0;
  return e(Ue, { className: c.root, children: a(_, { title: "Confirm account deletion", buttonColor: "regular", confirmButtonTitle: "Continue", fetching: l, disabled: f(), onOk: x, onClose: s, submitErrorText: p, noAutoFocus: true, children: [a(v, { variant: "body1", color: "primary", children: [e(q, { bottom: 25, children: "Deleting an account will remove all of the data associated with this account. You will have 30 days to contact us in case you need to reverse this process." }), "Please enter your ", D, " to confirm."] }), e(_e, { label: K(D), value: u, onChange: L, onBlur: L, error: w.password, type: "password", required: true, fullWidth: true, autoFocus: true, outlined: true })] }) });
}, os = m()({ root: { maxWidth: "573px" } }), as = I.promisify(ns), j = (t) => {
  const { classes: s } = is();
  return e(q, { top: 10, bottom: 5, children: e(je, { className: s.row, children: t.children }) });
}, is = m()({ row: { flexWrap: "wrap", "& button": { marginTop: "10px" } } }), Pe = (t) => {
  const { title: s, onClose: r, dark: o, textTheme: c } = t, l = Ut(), { classes: d } = cs();
  return e(Ge, { title: s, titleColor: "error", onClose: r, actionButtons: [{ children: "Contact support", onClick: l, variant: "text" }, { children: "Dismiss", onClick: r, color: "regular", autoFocus: true }], dark: o, textTheme: c, children: a(v, { children: ["Connection to the Termius sync server cannot be established. Please make sure:", a("ul", { className: d.list, children: [e("li", { children: "you have an active internet connection" }), e("li", { children: "your firewall allows internet access for the Termius app" }), e("li", { children: "your firewall allows HTTP requests to and responses from the api.termius.com server" })] })] }) });
}, cs = m()({ list: { "& > li": { listStyleType: "disc", marginLeft: "20px" } } });
Pe.defaultProps = { dark: false, textTheme: "primary" };
const ls = I.promisify(U(Pe));
function ds({ account: { two_factor_auth: t } }) {
  const s = E(), r = P(), o = () => {
    t ? (s(Ye()), O(r, { dialog: "OTPFlowDialog", target: W(), props: { action: "disable" } }).catch(Z)) : (s(Ke()), O(r, { dialog: "OTPFlowDialog", target: W(), props: { action: "enable" } }).catch(Z));
  };
  return e(T.Fragment, { children: e(xe, { checked: t, label: "Enable 2FA", onChange: o }) });
}
function us() {
  return a(T.Fragment, { children: [e(R, { children: "Локальная учётная запись" }), e(v, { color: "primary", children: "Локальный пользователь" }), e(k, { children: "Профиль постоянно активен. Вход в аккаунт Termius не требуется." }), e(k, { children: "Выход, смена аккаунта и удаление учётной записи отключены. Синхронизация работает через ваше облако." })] });
}
const hs = m()({ emailRow: { display: "flex", alignItems: "baseline", justifyContent: "space-between" }, buttonsRow: { display: "flex", alignItems: "center", justifyContent: "flex-end" }, buttonItem: { width: "56px" } }), ms = ({ isTeamOwner: t }) => {
  const s = P();
  return a(T.Fragment, { children: [e(R, { children: "Termius for Teams" }), t ? e(k, { children: "Enable group sharing to share groups and hosts with your team" }) : e(k, { children: "Enable group sharing to access the groups and hosts shared with your team" }), e(j, { children: e(H, { onClick: () => O(s, { dialog: "EnableSharingDialog", target: W(), props: {} }).catch(Z), size: "medium", children: "Enable group sharing" }) })] });
}, ae = (t) => {
  const { onClick: s, type: r } = t, { classes: o } = Cs(), c = i(J), l = a("div", { className: o.upgradeIconWrapper, children: [e("div", { className: o.pulseBackground }), e("div", { className: o.upgradeIcon, children: e(_t, {}) })] }), d = e(Qe, { className: z(o.linkIcon, o.inviteHover) }), u = { upgrade: { title: "Upgrade", icon: l }, buyNow: { title: "Buy now", icon: l }, manage: { title: "Manage", icon: d } };
  return c ? null : a("div", { className: o.upgradeButton, onClick: s, children: [e("div", { children: u[r].title }), u[r].icon] });
}, ps = N`
  0% {
    transform: scale(0.65);
    opacity: 0.5;
  }
  30% {
    opacity: 0.5;
  }
  95% {
    transform: scale(1.05);
    opacity: 0;
  }
  100% {
    transform: scale(1);
    opacity: 0;
  }
`, gs = N`
  0% {
    transform: scale(1);
  }
  50% {
    transform: scale(1.06);
  }
  100% {
    transform: scale(1);
  }
`, Cs = m()((t, s, r) => ({ upgradeButton: { display: "flex", alignItems: "center", gap: "8px", color: "var(--accent)", "&:hover": { color: "var(--accent-light)", cursor: "pointer" }, [`&:hover .${r.inviteHover}`]: { "& path": { color: "var(--accent-light)", fill: "var(--accent-light)" } } }, upgradeIconWrapper: { position: "relative", boxShadow: "0 0 0 0 rgba(#5a99d4, .5)", height: "21px" }, upgradeIcon: { position: "relative", zIndex: 1, borderRadius: "50%", width: "21px", height: "20px", background: "var(--card-c)", animation: `${gs} 1.6s forwards cubic-bezier(0.7, -0.3, 0.53, 1.26 ) infinite` }, pulseBackground: { position: "absolute", top: "-5px", right: "-5px", width: "31px", height: "30px", borderRadius: "50%", background: "linear-gradient(93.24deg, #2091F6 -0.06%, #21B568 99.94%)", outline: "1px solid transparent", animation: `${ps} 1.6s ease-in-out infinite` }, linkIcon: { marginTop: "2px", width: "14px", height: "14px", "& path": { fill: "var(--accent)" }, alignSelf: "center" }, inviteHover: {} })), ws = (t) => {
  const { isExpired: s, label: r, showLabel: o } = t, { classes: c } = fs();
  return o ? e("div", { className: z(c.label, { [c.expired]: s, [c.active]: !s }), children: r }) : null;
}, fs = m()({ label: { color: "white", letterSpacing: "0.5px", textTransform: "uppercase", fontWeight: 700, fontSize: "10px", lineHeight: "10px", padding: "6px 10px", borderRadius: "7px", marginLeft: "15px" }, expired: { background: "var(--red)" }, active: { background: "var(--accent)" } }), bs = () => {
  const { classes: t } = ys(), s = P(), r = i(et), o = i(fe).toLowerCase(), c = [tt, st].includes(o), l = i(Q), d = E(), u = async () => {
    d(ee({ highlighted: "no", upgradeButtonLocation: "Account Page", businessFunnelID: void 0 })), d(te({ choosePlanSource: "Upgrade Account" })), await oe({ sagas: s, isTriggeredByUser: true });
  };
  return r ? e(B, { children: a("div", { className: t.titleWrapper, children: [a(R, { children: [l, e(ws, { isExpired: c, label: c ? "Expired" : "Active" })] }), e(ae, { onClick: u, type: "upgrade" })] }) }) : null;
}, ys = m()({ titleWrapper: { display: "flex", alignItems: "center", justifyContent: "space-between" } }), Ss = (t, s = "LLL d, yyyy") => rt(t, s), xs = ({ isExpired: t, isTrial: s, isExpiredPlan: r }) => (t || !s) && r, vs = ({ isTeamMember: t, isSubscriptionWithRenew: s }) => t || s, Ts = ({ isSubscriptionWithRenew: t, memberInPaidTeam: s, isTrial: r, isExpiredSubscription: o }) => (t || s) && !r && !o, Ps = (t) => {
  const { useScaleAnimation: s } = t, { classes: r } = Ms();
  let o;
  const c = i(nt), l = c === "Trial" || c === "Team Trial Expired" || c === "Premium Team Trial Expired", d = i(ot), u = i(at);
  return l ? o = d : o = u, o !== 0 ? e("div", { className: z(r.progressBox, { scaleProgressBox: s }), children: e("div", { className: r.progressPercentage, style: { width: `${o}%`, animationDuration: `${Number(((100 - o) / 10).toFixed(1)) * 150}ms` }, children: e("div", { className: z(r.progressPercentageColor, { [r.animateProgressColorToYellow]: o >= 10 && o < 40, [r.animateProgressColorToRed]: o < 10 }), style: { animationDuration: `${Number(((100 - o) / 10).toFixed(1)) * 140}ms` } }) }) }) : null;
}, Es = N`
  to {
    background-position-x: 0%;
  }
`, Ls = N`
  to {
    background-position-x: 50%;
  }
`, Ds = N`
  0% {
    transform: scale3d(1, 1, 1);
  }
  15% {
    transform: scale3d(1.15, 1.65, 1);
  }
  75% {
    transform: scale3d(1.1, 1.6, 1);
  }
  100% {
    transform: scale3d(1, 1, 1);
  }
`, ks = N`
  to {
    min-width: 0%;
  }
`, Ms = m()({ progressBox: { marginTop: "10px", position: "relative", height: "5px", borderRadius: "2.5px", backgroundColor: "var(--dark-grey-7)", ".scaleProgressBox &": { animationName: Ds, animationDuration: "0.4s", animationDelay: "0.6s", animationFillMode: "forwards", animationTimingFunction: "ease-in" } }, progressPercentage: { position: "relative", overflow: "hidden", width: "100%", minWidth: "100%", height: "5px", borderRadius: "2.5px", animationName: ks, animationDuration: "1.5s", animationDelay: "1.2s", animationFillMode: "forwards", animationTimingFunction: "ease-out" }, progressPercentageColor: { "--progressColorAccent": "var(--accent)", ".sideBar &": { "--progressColorAccent": "var(--sidebar-accent)" }, position: "absolute", left: "0px", top: "0px", right: "0px", bottom: "0px", background: "linear-gradient(90deg, var(--red) 0%, var(--red) 20%, var(--yellow) 40%, var(--yellow) 60%, var(--progressColorAccent) 80%, var(--progressColorAccent) 100%)", backgroundSize: "3000px", backgroundPositionX: "100%", animationDuration: "1.5s", animationDelay: "1.2s", animationFillMode: "forwards", animationTimingFunction: "ease-out" }, animateProgressColorToRed: { animationName: Es }, animateProgressColorToYellow: { animationName: Ls } }), Ee = (t) => {
  const { classes: s } = As(), { expirationDate: r, activeText: o, isProgressLineHidden: c, showComponent: l } = t;
  return l ? a(k, { children: [e("div", { className: s.expirationContainer, children: o && a(v, { children: [o, !c && r] }) }), !c && e(q, { top: 9, children: e(Ps, {}) })] }) : null;
}, As = m()({ expirationContainer: { display: "flex", alignItems: "center", justifyContent: "space-between" } });
Ee.defaultProps = { isProgressLineHidden: false, progressPercentage: 0 };
const Bs = () => {
  var de;
  const { classes: t } = Fs(), s = P(), r = E(), o = i(it), c = i(fe).toLowerCase(), l = i(ct), d = i(se), u = i(J), C = i(be), p = i(lt), b = i(dt), w = i(ut), y = i(ht), L = !!i(mt), x = !!((de = i(he)) != null && de.auto_renew), h = i(pt), f = i(Q), g = i(ye), S = i(gt), V = i(Ct), M = xs({ isExpired: b, isTrial: !!w, isExpiredPlan: h }), G = !!i(he) && !M, ie = vs({ isTeamMember: u, isSubscriptionWithRenew: x }), Le = Ts({ isSubscriptionWithRenew: x, memberInPaidTeam: L, isTrial: !!w, isExpiredSubscription: b }), De = Ss(new Date((p == null ? void 0 : p.until) || Date.now())), ke = (() => Y().installSource !== "App Store" ? false : M && !d || y)(), Me = async () => {
    r(ee({ highlighted: "no", upgradeButtonLocation: "Account Page", businessFunnelID: void 0 })), r(te({ choosePlanSource: "Upgrade Account" })), await oe({ sagas: s, proPurchaseOnly: ke, isTriggeredByUser: true });
  }, ce = (() => Y().installSource === "App Store" ? G ? "manage" : "upgrade" : g ? "manage" : "upgrade")(), Ae = async () => {
    if (ce === "manage") {
      Se();
      return;
    }
    await Me();
  }, le = n.useMemo(() => {
    switch (c) {
      case wt:
        return a(T.Fragment, { children: ["Managed by:", a(v, { display: "inline", color: "primary", children: [" ", l] })] });
      default:
        return "Your subscription expires on ";
    }
  }, [l, c]);
  if (!o) return null;
  const Be = !Le && le, Fe = !y && !x && !M && !(!le && ie);
  return a(B, { children: [e("div", { className: t.titleWrapper, children: a(me, { children: [e($, { when: S, children: a(pe, { direction: "row", width: "100%", justifyContent: "space-between", children: [e(v, { variant: "subtitle1", color: "main", ellipsis: true, bolder: true, children: f }), e(ve, { onClick: ft, variant: "text", color: "accent", disableHover: true, className: t.manageButton, children: "Manage" })] }) }), e($, { when: !S, children: a(n.Fragment, { children: [e(R, { children: f }), e(ae, { onClick: Ae, type: ce })] }) })] }) }), a(me, { children: [e($, { when: S, children: a(pe, { direction: "row", gap: "7px", mt: "15px", children: [e(v, { children: "Managed by:" }), e(v, { color: "primary", children: V })] }) }), e($, { when: !S, children: e(Ee, { activeText: Be, expirationDate: De, isProgressLineHidden: ie, showComponent: Fe }) })] }), d && M && a(k, { children: ["Access to the ", e("strong", { children: "Team plan" }), " has been suspended", C.length ? a(T.Fragment, { children: ["for ", C.length, " team member", C.length > 1 && "s", "."] }) : "."] })] });
}, Fs = m()({ titleWrapper: { display: "flex", alignItems: "center", justifyContent: "space-between" }, manageButton: { padding: 0, height: "auto", "&:hover:not(:focus-visible):not(:disabled)": { color: "var(--accent-light)" }, "&:focus-visible": { background: "var(--accent-transparent)" } } }), Ns = Tt({ month: "short", day: "numeric", hour: "numeric", minute: "numeric" });
function Rs(t) {
  return t == null ? "" : Ns.format(new Date(t));
}
// Self-cloud status for this settings window. Statuses come from
// src/bridge/self-cloud.js (window.api.selfCloud); the server never sees
// decrypted data. "Получить данные" is offered only after a successful check.
const Hs_CONNECTED = ["synced", "upload", "download", "conflict", "uncertain", "empty"];
function Hs_host(url) {
  try { return new URL(url).host; } catch { return url || ""; }
}
function Hs_summary(t) {
  switch (t.kind) {
    case "synced": return "данные совпадают";
    case "upload": return `есть локальные изменения: ${t.upload || 0}`;
    case "download": return `на сервере новые данные: ${t.download || 0}`;
    case "conflict": return "изменения есть на обеих сторонах";
    case "uncertain": return "снимки отличаются";
    case "empty": return "снимок пуст";
    default: return "";
  }
}
function Hs() {
  const { classes: t } = Hs_styles(), r = window.api.selfCloud, [s, o] = T.useState(() => r.status()), [c, l] = T.useState(null);
  T.useEffect(() => {
    let d = true;
    const u = r.subscribe(o), C = () => r.config().then((p) => d && l(p), () => d && l({ configured: false }));
    C();
    // The settings window does not poll by itself: ask for a fresh status.
    r.check().catch(() => {});
    return window.addEventListener("focus", C), () => {
      d = false, u(), window.removeEventListener("focus", C);
    };
  }, []);
  T.useEffect(() => {
    r.config().then(l, () => {});
  }, [s.kind]);
  const d = c == null ? void 0 : c.configured, u = d && Hs_CONNECTED.includes(s.kind), C = Hs_host(c == null ? void 0 : c.url), p = c == null ? "Загрузка…" : !d ? "Сервер не настроен" : s.kind === "checking" || s.kind === "waiting" ? `Проверка подключения к ${C}…` : u ? `Подключено к ${C} · ${Hs_summary(s)}` : `Нет подключения к ${C}`, b = !d ? "idle" : u ? "ok" : s.kind === "error" ? "error" : "busy";
  return a(T.Fragment, { children: [e(R, { children: "Своё облако" }), e(k, { children: "Личная синхронизация с вашим сервером. Отправить и получить изменения также можно кнопками в верхней панели главного окна." }), a("div", { className: t.status, "data-state": b, children: [e("span", { className: t.dot, "aria-hidden": true }), e("span", { children: p })] }), d && s.kind === "error" && s.error ? a("div", { className: t.error, children: [s.error, s.credentials ? e("span", { className: t.hint, children: "Автоматическая проверка приостановлена до изменения настроек или ручной проверки." }) : null] }) : null, a("label", { className: t.toggle, children: [e("input", { type: "checkbox", checked: !!d && (c == null ? void 0 : c.autoSync) !== false, disabled: !d, onChange: (g) => void r.setAutoSync(g.target.checked).then(l, () => {}) }), e("span", { children: "Синхронизировать автоматически каждый час (изменения устройства и облака объединяются)" })] }), a("div", { className: t.actions, children: [e(H, { onClick: () => r.open(), size: "medium", children: d ? "Настройки подключения" : "Подключить сервер" }), d ? e(H, { onClick: () => r.open("sync"), size: "medium", children: "Синхронизировать" }) : null, u ? e(H, { onClick: () => r.open("download"), size: "medium", children: "Получить данные с сервера" }) : null, d ? e(H, { onClick: () => void r.check().catch(() => {}), size: "medium", disabled: s.kind === "checking", children: "Проверить" }) : null] })] });
}
// Tauri: delete session logs of the last hour/day/week or all of them, here
// and in the self-hosted cloud (src/bridge/self-cloud.js clearLogs).
const Hs_PERIODS = [["hour", "За час", "за последний час", 36e5], ["day", "За день", "за последний день", 864e5], ["week", "За неделю", "за последнюю неделю", 6048e5], ["all", "Все", "все", 0]];
function Hs_logs() {
  const { classes: t } = Hs_styles(), r = window.api.selfCloud, [s, o] = T.useState(null), [c, l] = T.useState(false), [d, u] = T.useState(null), C = Hs_PERIODS.find((p) => p[0] === s);
  const p = async () => {
    l(true), u(null);
    try {
      const b = await r.clearLogs(C[3] ? Date.now() - C[3] : 0);
      if (!(b != null && b.ok)) throw new Error((b == null ? void 0 : b.error) || "Не удалось удалить логи");
      const w = b.remote, y = w == null ? "" : w.error ? ` Облако: ${w.error}` : w.unsupported ? " Сервер не поддерживает удаление логов." : ` В облаке удалено логов: ${w.removed ?? 0}.`;
      u({ ok: true, text: `Удалено записей: ${b.entries}, лишних файлов: ${b.files}.${y}` });
    } catch (b) {
      u({ ok: false, text: (b == null ? void 0 : b.message) || String(b) });
    } finally {
      l(false), o(null);
    }
  };
  return a(T.Fragment, { children: [e(R, { children: "Логи сессий" }), e(k, { children: "Логи всего вывода терминала хранятся 60 дней. Очистка удаляет записи на экране Logs, файлы логов на этом устройстве и их копии в своём облаке за выбранный период." }), C ? a("div", { className: t.actions, children: [e("span", { className: t.confirm, children: `Удалить логи ${C[2]}? Это нельзя отменить.` }), e(H, { onClick: () => void p(), size: "medium", disabled: c, children: c ? "Удаление…" : "Удалить" }), e(H, { onClick: () => o(null), size: "medium", disabled: c, children: "Отмена" })] }) : e("div", { className: t.actions, children: Hs_PERIODS.map(([b, w]) => e(H, { onClick: () => (u(null), o(b)), size: "medium", children: w }, b)) }), d ? e("div", { className: d.ok ? t.notice : t.error, role: "status", children: d.text }) : null] });
}
const Hs_styles = m()({ status: { display: "flex", alignItems: "center", gap: "8px", marginTop: "12px", fontSize: "13px", lineHeight: "18px", color: "var(--text-primary)", '&[data-state="ok"] > span:first-of-type': { background: "var(--accent, #00cc74)" }, '&[data-state="error"] > span:first-of-type': { background: "#ec706f" }, '&[data-state="busy"] > span:first-of-type': { background: "#f0bc6b" } }, dot: { width: "8px", height: "8px", flexShrink: 0, borderRadius: "50%", background: "var(--text-secondary, #9ea5b3)" }, error: { marginTop: "8px", padding: "8px 10px", borderRadius: "6px", fontSize: "12px", lineHeight: "18px", color: "#ec706f", background: "rgba(236, 112, 111, 0.1)", overflowWrap: "anywhere", userSelect: "text" }, hint: { display: "block", marginTop: "4px", color: "var(--text-secondary, #9ea5b3)" }, actions: { display: "flex", flexWrap: "wrap", alignItems: "center", gap: "10px", marginTop: "16px" }, toggle: { display: "flex", alignItems: "center", gap: "8px", marginTop: "12px", fontSize: "13px", lineHeight: "18px", color: "var(--text-primary)", cursor: "pointer", "& input": { margin: 0, accentColor: "var(--accent, #00cc74)" } }, confirm: { fontSize: "13px", color: "var(--text-primary)" }, notice: { marginTop: "8px", padding: "8px 10px", borderRadius: "6px", fontSize: "12px", lineHeight: "18px", color: "var(--text-primary)", background: "rgba(0, 204, 116, 0.1)", overflowWrap: "anywhere", userSelect: "text" } });
const Is = m()({ syncDate: { color: "var(--dark-grey-1)", ".termius-dark-theme &": { color: "white" } } }), Vs = () => {
  const [t] = i(Pt);
  return t < 1 ? null : e(v, { variant: "body1", color: "secondary", children: `${t} day${t === 1 ? "" : "s"} left` });
}, $s = () => {
  const t = E(), s = P(), { classes: r } = Os(), o = i(Et), c = i(Lt), l = i(Dt), d = i(se), u = i(be), C = i(kt), p = i(Mt) || C && !c, b = i(J), w = i(ye), y = i(At), L = i(Q), x = d && p, h = c && !d, f = i(Bt), g = i(Ft), S = n.useCallback(() => {
    Nt({ username: l, plan: "pro" });
  }, [l]), V = async () => {
    t(ee({ highlighted: "no", upgradeButtonLocation: "Account Page", businessFunnelID: void 0 })), t(te({ choosePlanSource: "Upgrade Account" })), await oe({ sagas: s, proPurchaseOnly: g, isTriggeredByUser: true });
  }, M = (() => h ? "buyNow" : Y().installSource === "App Store" ? f && c || y ? "manage" : "upgrade" : w && y ? "manage" : "upgrade")(), G = () => {
    if (M === "manage") {
      Se();
      return;
    }
    V();
  };
  return o ? a(B, { children: [a("div", { className: r.trialWrapper, children: [a("div", { className: r.titleWrapper, children: [e(R, { children: L }), e(Vs, {})] }), e(ae, { onClick: G, type: M })] }), x && a(k, { children: ["Access to the ", e("strong", { children: "Team plan" }), " has been suspended", u.length ? a(T.Fragment, { children: [" ", "for ", u.length, " team member", u.length > 1 && "s", "."] }) : "."] }), b && e(j, { children: e(ve, { size: "medium", onClick: S, children: "Manage plan" }) })] }) : null;
}, Os = m()({ trialWrapper: { display: "flex", alignItems: "center", justifyContent: "space-between" }, titleWrapper: { display: "flex", gap: "10px" } }), Ws = (t) => {
  const { user: s } = t;
  return a(T.Fragment, { children: [e(B, { children: e(us, { user: s }) }), e(B, { children: e(Hs, { user: s }) }), e(B, { children: e(Hs_logs, {}) })] });
}, Zs = (t, s) => n.createElement("svg", { width: 30, height: 31, viewBox: "0 0 30 31", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: s, ...t }, n.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M7.95837 4.13414C7.70239 4.22838 7.24967 4.53317 6.95228 4.81143C6.64098 5.10295 5.0525 7.25143 3.20587 9.87871L0 14.44L0.00108677 19.3896L0.00217396 24.3393L0.260326 24.8938C0.564674 25.5474 1.13011 26.1437 1.76924 26.4852L2.22826 26.7306H15H27.7717L28.2191 26.4912C28.9476 26.1015 29.3153 25.729 29.6676 25.024L30 24.3589V19.4361V14.5132L26.7332 9.83784C24.7601 7.01404 23.2726 4.99197 22.9772 4.73186C21.5313 3.45914 19.379 3.76545 18.1585 5.41762L17.7105 6.02404H16.1329C14.6029 6.02404 14.5473 6.03208 14.2886 6.29088C13.9647 6.61469 13.9402 7.09491 14.2293 7.45219C14.4364 7.70806 14.445 7.70893 17.2063 7.7411C19.7721 7.7711 19.9895 7.78871 20.1637 7.98132C20.4171 8.26132 20.2982 8.7161 19.9355 8.85393C19.7839 8.91164 18.7863 8.95882 17.7188 8.95882H15.7778L15.3879 9.41436C15.1553 9.6861 14.9604 9.8286 14.905 9.76762C14.8538 9.71132 14.1638 8.73599 13.3715 7.60012C11.5117 4.93349 11.333 4.72034 10.6416 4.34393C9.92457 3.95371 8.70783 3.8586 7.95837 4.13414ZM9.88935 5.94251C10.1068 6.09175 11.4674 7.92371 13.3809 10.6436L16.5161 15.1001L16.5189 19.4243C16.522 24.1021 16.5133 24.1808 15.9292 24.7197L15.6348 24.9914L9.22989 25.0211C2.14674 25.0539 2.38793 25.0759 1.95217 24.3611C1.75076 24.0309 1.73989 23.7847 1.73957 19.5223L1.73913 15.0315L4.91196 10.5204C6.73598 7.92708 8.20304 5.94501 8.36304 5.85806C8.79522 5.62295 9.47902 5.66077 9.88935 5.94251ZM21.5907 5.90654C21.778 6.02077 22.0554 6.27741 22.2072 6.47675C22.8262 7.29012 27.3846 13.8018 27.3878 13.8773C27.3897 13.9221 25.8873 13.957 24.0489 13.9547L20.7065 13.9506L20.4302 13.5471C20.2782 13.3251 19.8928 12.7683 19.5737 12.3096L18.9936 11.4757L19.2198 11.1097C19.4118 10.7989 19.5425 10.7237 20.0842 10.6118C21.9789 10.2204 22.6999 8.0648 21.3777 6.74458C21.1029 6.47023 20.7309 6.2324 20.4348 6.14197L19.9457 5.99262L20.2717 5.85023C20.7368 5.64697 21.1974 5.66664 21.5907 5.90654ZM8.58696 6.97012C8.2825 7.08088 7.83152 7.58023 7.7138 7.93708C7.42446 8.81382 8.12913 9.75588 9.07435 9.75588C10.5565 9.75588 11.0953 7.73545 9.79283 7.06197C9.47283 6.89643 8.90761 6.85338 8.58696 6.97012ZM28.2604 19.8556C28.2601 23.7315 28.2462 24.0359 28.0559 24.3479C27.6349 25.0384 27.5764 25.0458 22.5034 25.0458H17.9201L17.9986 24.8012C18.2237 24.0995 18.2559 23.4731 18.2583 19.7469L18.2609 15.6979H23.2609H28.2609L28.2604 19.8556Z", fill: "currentColor" })), zs = n.forwardRef(Zs), Us = n.memo(zs), _s = (t, s) => n.createElement("svg", { width: 30, height: 26, viewBox: "0 0 30 26", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: s, ...t }, n.createElement("mask", { id: "path-1-inside-1_53_2065", fill: "white" }, n.createElement("path", { d: "M30 14.554C30 18.8441 26.5307 22.3327 22.2644 22.3327C22.1603 22.3327 22.056 22.3305 21.9516 22.3263C20.4772 24.577 17.9779 25.9465 15.2674 25.9465C12.4335 25.9465 9.81468 24.4242 8.38239 22.0014C8.17996 22.0172 7.97677 22.0252 7.77373 22.0252C3.48637 22.0252 0 18.5193 0 14.208C0 11.3594 1.54025 8.75485 3.98343 7.38204C4.15492 3.37115 7.45204 0.16272 11.4823 0.16272C13.4143 0.16272 15.2459 0.902206 16.6321 2.2187C17.6856 1.67633 18.8503 1.39293 20.0466 1.39293C24.1658 1.39293 27.5192 4.74438 27.5525 8.87862C29.0961 10.3403 30 12.4181 30 14.554Z" })), n.createElement("path", { d: "M27.5525 8.87862L25.7525 8.89309L25.7587 9.65901L26.3149 10.1856L27.5525 8.87862ZM16.6321 2.2187L15.3925 3.52386L16.3195 4.40421L17.456 3.81905L16.6321 2.2187ZM3.98343 7.38204L4.86517 8.95128L5.73897 8.4603L5.78178 7.45893L3.98343 7.38204ZM8.38239 22.0014L9.9319 21.0854L9.3608 20.1193L8.24197 20.2068L8.38239 22.0014ZM21.9516 22.3263L22.0249 20.5278L21.0051 20.4862L20.4459 21.34L21.9516 22.3263ZM22.2644 24.1327C27.5342 24.1327 31.8 19.8288 31.8 14.554H28.2C28.2 17.8594 25.5272 20.5327 22.2644 20.5327V24.1327ZM31.8 14.554C31.8 11.9193 30.6896 9.37024 28.7901 7.57159L26.3149 10.1856C27.5027 11.3104 28.2 12.9169 28.2 14.554H31.8ZM29.3524 8.86414C29.3113 3.75163 25.1644 -0.407072 20.0466 -0.407072V3.19293C23.1672 3.19293 25.7271 5.73713 25.7525 8.89309L29.3524 8.86414ZM20.0466 -0.407072C18.5639 -0.407072 17.1167 -0.0553267 15.8082 0.618344L17.456 3.81905C18.2545 3.40798 19.1367 3.19293 20.0466 3.19293V-0.407072ZM17.8717 0.913532C16.153 -0.718788 13.8784 -1.63728 11.4823 -1.63728V1.96272C12.9503 1.96272 14.3389 2.5232 15.3925 3.52386L17.8717 0.913532ZM11.4823 -1.63728C6.47451 -1.63728 2.39718 2.34429 2.18507 7.30515L5.78178 7.45893C5.91266 4.398 8.42956 1.96272 11.4823 1.96272V-1.63728ZM3.10168 5.8128C0.0923377 7.50373 -1.8 10.7076 -1.8 14.208H1.8C1.8 12.0111 2.98815 10.006 4.86517 8.95128L3.10168 5.8128ZM-1.8 14.208C-1.8 19.5039 2.48281 23.8252 7.77373 23.8252V20.2252C4.48992 20.2252 1.8 17.5347 1.8 14.208H-1.8ZM7.77373 23.8252C8.02375 23.8252 8.27378 23.8154 8.52282 23.7959L8.24197 20.2068C8.08614 20.219 7.92979 20.2252 7.77373 20.2252V23.8252ZM6.83289 22.9174C8.58399 25.8795 11.7871 27.7465 15.2674 27.7465V24.1465C13.0798 24.1465 11.0454 22.9689 9.9319 21.0854L6.83289 22.9174ZM15.2674 27.7465C18.5911 27.7465 21.6537 26.0659 23.4573 23.3126L20.4459 21.34C19.3007 23.0881 17.3648 24.1465 15.2674 24.1465V27.7465ZM21.8782 24.1248C22.0069 24.13 22.1357 24.1327 22.2644 24.1327V20.5327C22.185 20.5327 22.1051 20.5311 22.0249 20.5278L21.8782 24.1248Z", fill: "currentColor", mask: "url(#path-1-inside-1_53_2065)" }), n.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M15.1183 6.97237C13.6672 6.97237 12.4908 8.14872 12.4908 9.59982V10.6508H13.5418V9.59982C13.5418 8.72916 14.2476 8.02335 15.1183 8.02335C15.9889 8.02335 16.6948 8.72916 16.6948 9.59982V10.6508H17.7457V9.59982C17.7457 8.14872 16.5694 6.97237 15.1183 6.97237ZM11.4399 11.702H18.7968V16.4314C18.7968 17.0118 18.3262 17.4824 17.7458 17.4824H12.4909C11.9104 17.4824 11.4399 17.0118 11.4399 16.4314V11.702ZM15.4937 14.6794C15.7181 14.5496 15.869 14.307 15.869 14.0291C15.869 13.6145 15.5329 13.2784 15.1183 13.2784C14.7037 13.2784 14.3676 13.6145 14.3676 14.0291C14.3676 14.307 14.5186 14.5496 14.743 14.6794V15.5305C14.743 15.7378 14.911 15.9059 15.1183 15.9059C15.3256 15.9059 15.4937 15.7378 15.4937 15.5305V14.6794Z", fill: "currentColor" })), qs = n.forwardRef(_s), js = n.memo(qs), Gs = (t, s) => n.createElement("svg", { width: 30, height: 31, viewBox: "0 0 30 31", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: s, ...t }, n.createElement("g", { id: "Group 3" }, n.createElement("g", { id: "Combined Shape phone" }, n.createElement("path", { id: "Subtract", fillRule: "evenodd", clipRule: "evenodd", d: "M0 6.9467C0 5.28985 1.34315 3.9467 3 3.9467H14.25C14.6642 3.9467 15 4.28249 15 4.6967C15 5.11091 14.6642 5.4467 14.25 5.4467H3C2.17157 5.4467 1.5 6.11827 1.5 6.9467V18.9467C1.5 19.7751 2.17157 20.4467 3 20.4467H13.5C14.3284 20.4467 15 21.1183 15 21.9467V22.9852C15 23.8136 14.3284 24.4852 13.5 24.4852H3C1.34314 24.4852 0 23.142 0 21.4852V6.9467ZM15 25.8124C15 25.3982 14.6642 25.0624 14.25 25.0624H12.7872C12.1614 25.0624 11.6759 25.5956 11.3961 26.1554L11.1592 26.6291C11.1091 26.7292 11.0069 26.7924 10.895 26.7924C10.2993 26.7924 9.75478 27.129 9.48838 27.6618C9.42303 27.7925 9.51808 27.9462 9.66421 27.9462H14.25C14.6642 27.9462 15 27.6105 15 27.1962V25.8124Z", fill: "currentColor" }), n.createElement("path", { id: "Union", fillRule: "evenodd", clipRule: "evenodd", d: "M27.975 3.9465H18.525C17.4066 3.9465 16.5 4.82471 16.5 5.90804V25.985C16.5 27.0683 17.4066 27.9465 18.525 27.9465H27.975C29.0934 27.9465 30 27.0683 30 25.985V5.90804C30 4.82471 29.0934 3.9465 27.975 3.9465ZM18 6.08936C18 5.73432 18.2612 5.4465 18.5833 5.4465H19.5C19.5 6.27493 20.1716 6.9465 21 6.9465H25.5C26.3284 6.9465 27 6.27493 27 5.4465H27.9167C28.2388 5.4465 28.5 5.73432 28.5 6.08936V25.8036C28.5 26.1587 28.2388 26.4465 27.9167 26.4465H18.5833C18.2612 26.4465 18 26.1587 18 25.8036V6.08936Z", fill: "currentColor" })))), Ys = n.forwardRef(Gs), Ks = n.memo(Ys), Xs = "" + new URL("welcome@1x-7eaeda4c.webp", import.meta.url).href, Js = "" + new URL("welcomeDark@1x-d6183d2a.webp", import.meta.url).href, Qs = () => {
  const { classes: t } = er(), s = i(we);
  return a("div", { className: t.container, children: [e("div", { className: t.title, children: "Try Pro at no cost" }), a("div", { className: t.subtitle, children: ["Join over ", e("strong", { children: "22,000" }), " professionals", e("br", {}), "who save more than ", e("strong", { children: "6 hours and 19 minutes" }), e("br", {}), "on connections every month"] }), e("img", { className: t.image, src: s ? Js : Xs, alt: "" }), a("div", { className: t.features, children: [a("div", { className: t.feature, children: [e(js, {}), " ", a("div", { children: [e("strong", { children: "Backups" }), " in a secure vault"] })] }), a("div", { className: t.feature, children: [e(Ks, {}), " ", a("div", { children: [e("strong", { children: "Sync" }), " on unlimited devices ", e("br", {}), "on all platforms"] })] }), a("div", { className: t.feature, children: [e(Us, {}), " ", a("div", { children: [e("strong", { children: "Advanced settings:" }), " Agent Forwarding, ", e("br", {}), "Jump Host, HTTP Proxy"] })] })] })] });
}, er = m()({ container: { "--title-color": "var(--dark-grey-1)", "--text-color": "var(--light-grey-1)", ".termius-dark-theme &": { "--title-color": "var(--white)", "--text-color": "var(--dark-grey-7)" }, textAlign: "center", display: "flex", flexDirection: "column", alignItems: "center" }, title: { color: "var(--title-color)", fontSize: "24px", fontWeight: 700, lineHeight: "30px" }, subtitle: { color: "var(--text-color)", fontSize: "14px", lineHeight: "21px", marginTop: "10px" }, features: { marginLeft: "45px" }, feature: { display: "flex", color: "var(--text-color)", fontSize: "14px", alignItems: "center", textAlign: "left", marginBottom: "20px", "& svg": { marginRight: "15px", color: "var(--accent)" } }, image: { width: "227px", margin: "10px 0" } }), tr = n.lazy(() => Ht(() => import("./AuthorizationForm-253327c1.js").then((t) => t.b), ["./AuthorizationForm-253327c1.js", "./reconnectSaga-3b11f62e.js", "./reconnectSaga-197a1ef0.css", "./OTPToken-a5994a9f.js", "./InputDialog-b3f32c9f.js", "./AlertDialog-b50b9ed7.js", "./Button-90700c84.js", "./PasswordInput-9d57733c.js", "./OtpLogin-e15265bf.js"], import.meta.url)), sr = () => {
  const [t, s] = n.useState(false), r = E(), o = n.useCallback(() => {
    s(true);
  }, []);
  n.useEffect(() => {
    r(It({ createAccountScreenType: "Preferences Account Screen", upgradeToProFunnelID: Vt() }));
  }, [r]);
  const { classes: c } = rr();
  return e("div", { className: c.container, children: a("div", { className: c.group, children: [e("div", { className: c.form, children: e(n.Suspense, { fallback: "", children: e(tr, { from: "Settings", onSsoDisplayed: o }) }) }), !t && e("div", { className: c.promo, children: e(Qs, {}) })] }) });
}, rr = m()({ container: { position: "absolute", inset: 0, overflow: "auto", background: "var(--light-grey-6)", ".termius-dark-theme &": { background: "var(--dark-grey-1)" } }, group: { display: "grid", width: "100%", height: "100%", justifyContent: "space-evenly", paddingTop: 60 }, form: { maxWidth: "440px", gridRow: 1 }, promo: { gridRow: 1 } }), Sr = () => {
  const t = i($t), s = i(Ot), r = i(se);
  return e(Wt, { children: (
    /* self-cloud:local-account-route */
    e(Ws, { user: t, isTeam: false, isTeamOwner: false })
  ) });
};
export {
  Sr as Profile
};
