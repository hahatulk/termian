/*
 * Recovered Termius UI module: SnackbarController-d2c726d8.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { vF as q, vG as X, hO as J, bO as D, vH as Q, hM as Y, r as f, bi as k, hE as x, bt as i, br as h, bP as _, bu as b, dl as Z, cv as $, vI as ee, bq as N, dr as oe, aH as T, vJ as te, vK as ne, eS as se, vL as re, vM as ae, dE as ie, s9 as O, vN as U, bs as C, vO as P, jw as ce, hH as le, i2 as V, vP as de, bj as E, uH as ue, vQ as L, f3 as pe, pr as z, uA as ve, vR as he, ca as fe, gq as ge, gr as me, vS as xe, vT as Ce, gt as be, be as ye, ii as ke, cN as Se, vU as we, f2 as B, f1 as Ae, ix as Ee, iK as Ne, sj as Te, fu as Me, vV as Ie, vW as De, iy as $e } from "./reconnectSaga-3b11f62e.js";
import { C as Le } from "./ClickAwayListener-fa29c305.js";
function Re(o, n) {
  return o.classList ? !!n && o.classList.contains(n) : (" " + (o.className.baseVal || o.className) + " ").indexOf(" " + n + " ") !== -1;
}
function Pe(o, n) {
  o.classList ? o.classList.add(n) : Re(o, n) || (typeof o.className == "string" ? o.className = o.className + " " + n : o.setAttribute("class", (o.className && o.className.baseVal || "") + " " + n));
}
function W(o, n) {
  return o.replace(new RegExp("(^|\\s)" + n + "(?:\\s|$)", "g"), "$1").replace(/\s+/g, " ").replace(/^\s*|\s*$/g, "");
}
function ze(o, n) {
  o.classList ? o.classList.remove(n) : typeof o.className == "string" ? o.className = W(o.className, n) : o.setAttribute("class", W(o.className && o.className.baseVal || "", n));
}
var Be = function(n, a) {
  return n && a && a.split(" ").forEach(function(e) {
    return Pe(n, e);
  });
}, R = function(n, a) {
  return n && a && a.split(" ").forEach(function(e) {
    return ze(n, e);
  });
}, F = (function(o) {
  q(n, o);
  function n() {
    for (var e, u = arguments.length, c = new Array(u), d = 0; d < u; d++) c[d] = arguments[d];
    return e = o.call.apply(o, [this].concat(c)) || this, e.appliedClasses = { appear: {}, enter: {}, exit: {} }, e.onEnter = function(r, s) {
      var l = e.resolveArguments(r, s), p = l[0], t = l[1];
      e.removeClasses(p, "exit"), e.addClass(p, t ? "appear" : "enter", "base"), e.props.onEnter && e.props.onEnter(r, s);
    }, e.onEntering = function(r, s) {
      var l = e.resolveArguments(r, s), p = l[0], t = l[1], v = t ? "appear" : "enter";
      e.addClass(p, v, "active"), e.props.onEntering && e.props.onEntering(r, s);
    }, e.onEntered = function(r, s) {
      var l = e.resolveArguments(r, s), p = l[0], t = l[1], v = t ? "appear" : "enter";
      e.removeClasses(p, v), e.addClass(p, v, "done"), e.props.onEntered && e.props.onEntered(r, s);
    }, e.onExit = function(r) {
      var s = e.resolveArguments(r), l = s[0];
      e.removeClasses(l, "appear"), e.removeClasses(l, "enter"), e.addClass(l, "exit", "base"), e.props.onExit && e.props.onExit(r);
    }, e.onExiting = function(r) {
      var s = e.resolveArguments(r), l = s[0];
      e.addClass(l, "exit", "active"), e.props.onExiting && e.props.onExiting(r);
    }, e.onExited = function(r) {
      var s = e.resolveArguments(r), l = s[0];
      e.removeClasses(l, "exit"), e.addClass(l, "exit", "done"), e.props.onExited && e.props.onExited(r);
    }, e.resolveArguments = function(r, s) {
      return e.props.nodeRef ? [e.props.nodeRef.current, r] : [r, s];
    }, e.getClassNames = function(r) {
      var s = e.props.classNames, l = typeof s == "string", p = l && s ? s + "-" : "", t = l ? "" + p + r : s[r], v = l ? t + "-active" : s[r + "Active"], g = l ? t + "-done" : s[r + "Done"];
      return { baseClassName: t, activeClassName: v, doneClassName: g };
    }, e;
  }
  var a = n.prototype;
  return a.addClass = function(u, c, d) {
    var r = this.getClassNames(c)[d + "ClassName"], s = this.getClassNames("enter"), l = s.doneClassName;
    c === "appear" && d === "done" && l && (r += " " + l), d === "active" && u && X(u), r && (this.appliedClasses[c][d] = r, Be(u, r));
  }, a.removeClasses = function(u, c) {
    var d = this.appliedClasses[c], r = d.base, s = d.active, l = d.done;
    this.appliedClasses[c] = {}, r && R(u, r), s && R(u, s), l && R(u, l);
  }, a.render = function() {
    var u = this.props;
    u.classNames;
    var c = J(u, ["classNames"]);
    return D.createElement(Q, Y({}, c, { onEnter: this.onEnter, onEntered: this.onEntered, onEntering: this.onEntering, onExit: this.onExit, onExiting: this.onExiting, onExited: this.onExited }));
  }, n;
})(D.Component);
F.defaultProps = { classNames: "" };
F.propTypes = {};
const Fe = F;
function He(o) {
  return o.snackbar;
}
function j(o) {
  const [n, a] = f.useState(/* @__PURE__ */ new Date());
  return f.useEffect(() => {
    const e = setInterval(() => {
      a(/* @__PURE__ */ new Date());
    }, o);
    return () => {
      clearInterval(e);
    };
  }, [o]), n;
}
const We = (o) => {
  const { delay: n, onTimerEnd: a } = o, [e, u] = f.useState(n);
  return f.useEffect(() => {
    let c;
    const d = () => (clearInterval(c), a(), 0);
    return c = setInterval(() => {
      u((s) => s <= 1 ? d() : s - 1);
    }, 1e3), () => {
      c && clearTimeout(c);
    };
  }, [e]), e;
};
function _e(o) {
  const { colorMode: n } = o, { classes: a } = Oe(), e = { main: a.notificationMain, adaptive: a.notificationAdaptive };
  return { error: a.notificationAlert, regular: e[n] };
}
const Oe = k()({ notificationMain: { "--notification-text-color": "var(--white)", "--notification-text-color-hover": "var(--white-a50)", "--notification-border-color": "var(--light-grey-a15)", "--notification-background-color": "var(--dark-grey-3)", "--notification-button-background-color": "var(--white)", "--notification-button-text-color": "var(--dark-grey-3)", "--notification-button-text-color-hover": "var(--notification-button-text-color)", "--notification-button-background-color-hover": "var(--white-a75)", "--hotkey-text-color": "var(--dark-grey-1)", "--hotkey-background-color": "var(--dark-grey-7)" }, notificationAlert: { "--notification-text-color": "var(--white)", "--notification-text-color-hover": "var(--white-a50)", "--notification-border-color": "var(--light-grey-a15)", "--notification-background-color": "var(--red)", "--notification-button-background-color": "var(--white)", "--notification-button-text-color": "var(--red)", "--notification-button-text-color-hover": "var(--notification-button-text-color)", "--notification-button-background-color-hover": "var(--white-a75)", "--hotkey-text-color": "var(--dark-grey-1)", "--hotkey-background-color": "var(--dark-grey-7)" }, notificationAdaptive: { "--notification-text-color": x.uiColor, "--notification-text-color-hover": x.foregroundColor, "--notification-border-color": "transparent", "--notification-background-color": x.uiColorA20, "--notification-button-background-color": x.uiColorA10, "--notification-button-text-color": x.uiColor, "--notification-button-text-color-hover": x.foregroundColor, "--notification-button-background-color-hover": x.uiColorA10, "--hotkey-text-color": x.uiColor, "--hotkey-background-color": x.uiColorA25 } }), S = f.forwardRef((o, n) => {
  const { className: a, children: e, actions: u, type: c = "regular", timer: d = 5e3, handlers: r, onClose: s, onClickAway: l = s, onTimerEnd: p = s, colorMode: t = "main" } = o, { classes: v } = Ue(), y = _e({ colorMode: t })[c];
  return We({ delay: Math.round(d / 1e3), onTimerEnd: p }), i(Le, { onClickAway: l, mouseEvent: "onPointerDown", children: h("div", { ref: n, tabIndex: -1, role: "presentation", className: _("termius-dark-theme", v.container, a, y), children: [h("div", { children: [i("div", { className: v.message, children: i(b, { variant: "body2", fontSize: 12, children: e }) }), u && i(Z, { children: u.map((w, m) => i($, { size: "small", className: v.button, onClick: r == null ? void 0 : r[m], disableHover: true, children: i(b, { variant: "mediumBody1", fontSize: 14, children: w }) }, `${w}-${m}`)) })] }), i("button", { "data-testid": "notification-close", type: "button", className: v.close, onClick: s, children: i(ee, {}) })] }) });
}), Ue = k()({ container: { position: "relative", width: 302, borderRadius: 10, border: "1px solid var(--notification-border-color)", background: "var(--notification-background-color)", color: "var(--notification-text-color)", padding: "9px", boxShadow: "0px 15px 30px 0px rgba(0, 0, 0, 0.15)", backdropFilter: "blur(50px)", marginTop: 5 }, message: { marginRight: "15px", wordWrap: "break-word" }, button: { padding: "5px 10px", background: "var(--notification-button-background-color)", color: "var(--notification-button-text-color)", borderRadius: "6px", width: "100%", marginTop: 10, textTransform: "capitalize", "&:hover": { "--notification-button-text-color": "var(--notification-button-text-color-hover)", "--notification-button-background-color": "var(--notification-button-background-color-hover)" } }, close: { color: "var(--notification-text-color)", position: "absolute", width: "12px", height: "12px", right: "8px", top: "11px", border: "none", backgroundColor: "transparent", display: "flex", zIndex: 2, "& svg": { height: "12px", width: "12px" }, "&:hover": { "--notification-text-color": "var(--notification-text-color-hover)" } } });
function Ve({ title: o, message: n, ...a }) {
  const { classes: e } = je();
  return i(S, { ...a, children: h("div", { className: e.content, children: [i("div", { className: e.title, children: o }), i("div", { className: e.message, children: n })] }) });
}
const je = k()({ content: { display: "flex", flexDirection: "column" }, title: { fontSize: "14px" }, message: { fontSize: "10px" } });
function Ke({ association: o, ...n }) {
  const a = N(), e = () => {
    a(te(o)), a(ne({ onScreen: "SFTP" })), n.onClose();
  }, u = oe.basename(o.appPath);
  return h(S, { ...n, handlers: [e, n.onClose], onClickAway: T, children: ["Always use ", i("strong", { children: u }), " to open", " ", h("strong", { children: [".", o.type] }), " files?"] });
}
function Ge({ message: o, ...n }) {
  return i(S, { ...n, children: o });
}
function K(o, n) {
  se(2, arguments);
  var a = ae(n);
  return re(o, a * 1e3);
}
function qe(o) {
  return o < 33 ? "red" : o < 66 ? "yellow" : "green";
}
const G = ({ countdown: o, progress: n }) => {
  const { classes: a } = Xe();
  return h("div", { className: a.root, children: [i(b, { className: a.text, align: "center", fontSize: 20, children: o }), i(ie, { className: _(a.progress, a[qe(n)]), thickness: 3, size: 34, variant: "determinate", value: n })] });
}, Xe = k()({ root: { position: "relative", display: "flex", width: "34px", height: "34px" }, text: { lineHeight: "34px", width: "34px", margin: "auto", color: "var(--white)" }, red: { color: "var(--red)", "& svg": { color: "var(--red)" } }, yellow: { color: "var(--yellow)", "& svg": { color: "var(--yellow)" } }, green: { color: "var(--green)", "& svg": { color: "var(--green)" } }, progress: { transform: "scale(-1, 1)", position: "absolute" } });
function Je(o) {
  const { message: n, createdAt: a, id: e, target: u, timer: c } = o, [d] = f.useState(+/* @__PURE__ */ new Date()), { classes: r } = Qe(), s = N(), l = () => {
    s(P().processed({ id: e })), o.onClose();
  }, p = () => {
    s(P().canceled({ id: e })), o.onClose();
  }, t = O(+a + c - d, 1, c), v = Math.round(t / 1e3), g = j(t / 100), y = K(a, v), m = Math.max(U(y, g), 0) / t;
  return i(S, { ...o, colorMode: "main", onClose: l, children: h(C, { direction: "row", children: [i(G, { countdown: Math.ceil(v * m), progress: m * 100 }), h("div", { className: r.content, children: [h(b, { display: "inline", color: "inherit", lineHeight: 1, fontSize: 12, children: [n, " ", u] }), i($, { size: "small", className: r.button, onClick: p, children: i(b, { variant: "mediumBody1", fontSize: 14, children: "Undo" }) })] })] }) });
}
const Qe = k()({ content: { marginLeft: "10px", width: "100%" }, button: { textTransform: "none", marginTop: "10px", backgroundColor: "var(--dark-grey-6)" } });
function Ye({ id: o, createdAt: n, timer: a, onClose: e, currentWindowFocused: u }) {
  const c = N(), [d, r] = f.useState(false), s = V(de), l = u && d;
  f.useEffect(() => {
    d || r(s);
  }, [s, d]);
  const { length: p } = E(ue), t = f.useRef(null);
  f.useEffect(() => {
    p < 1 && (c(L.canceled({ id: o })), e());
  }, [c, o, e, p]);
  const v = () => {
    c(L.canceled({ id: o })), e();
  }, g = (A) => {
    c(L.processed({ id: o, data: A })), e();
  }, y = () => {
    g({ connectionOrigin: "Reconnect Button" });
  }, w = () => {
    g({ connectionOrigin: "Reconnect Hotkey" });
  }, m = () => {
    c(z({ closingMethod: "Esc", hintLevel: "Warn", hintMessage: "Disconnect" })), v();
  }, H = () => {
    c(z({ closingMethod: "Button", hintLevel: "Warn", hintMessage: "Disconnect" })), v();
  }, M = () => {
    c(P().canceled({ id: o })), c(ve()), e();
  };
  return f.useEffect(() => {
    var I;
    const A = document.activeElement;
    return u && d && ((I = t.current) == null || I.focus()), () => {
      d && (A == null || A.focus());
    };
  }, [u, d]), i(pe.GlobalHotKeys, { keyMap: { onEsc: "escape", onEnter: "enter" }, handlers: { onEsc: m, onEnter: w }, children: i(Ze, { ref: t, createdAt: n, timer: a, onTimerEnd: e, onCloseButtonClick: H, onReconnectButtonClick: y, onCloseDisconnectedButtonClick: M, reconnectableTerminalCount: p, visible: l }) });
}
const Ze = f.forwardRef(({ createdAt: o, timer: n, onCloseButtonClick: a, onCloseDisconnectedButtonClick: e, onReconnectButtonClick: u, onTimerEnd: c, reconnectableTerminalCount: d, visible: r, ...s }, l) => {
  const { classes: p, cx: t } = eo(), [v] = f.useState(Date.now()), g = O(o.valueOf() + n - v, 1, n), y = Math.round(g / 1e3), w = j(g / 100), m = K(o, y), M = Math.max(U(m, w), 0) / g, A = d === 1 ? "1 session is disconnected" : `${d} sessions are disconnected`, I = d === 1 ? "Close terminal" : "Close terminals";
  return i(S, { ...s, ref: l, timer: n, onClose: a, onClickAway: T, className: t({ [p.invisible]: !r, [p.visible]: r }), onTimerEnd: c, colorMode: "main", children: i(C, { gap: "10px", children: h(C, { direction: "row", gap: "10px", children: [i(G, { countdown: Math.ceil(y * M), progress: M * 100 }), h(C, { gap: "10px", children: [h(b, { variant: "body2", fontSize: 12, children: [A, ". ", i("br", {}), " Initiating reconnection..."] }), h(C, { direction: "row", gap: "10px", children: [i($, { size: "small", "aria-label": "Force reconnect terminals", color: "accent", variant: "contained", className: p.reconnectButton, onClick: u, children: i(b, { variant: "mediumBody1", fontSize: 14, children: h(C, { direction: "row", gap: "5px", alignItems: "center", children: ["Reconnect", i(ce, { width: 14, height: 14 })] }) }) }), i($, { size: "small", "aria-label": "Close all disconnected terminals", variant: "contained", className: p.closeTerminalButton, onClick: e, children: i(b, { variant: "mediumBody1", fontSize: 14, children: i(C, { direction: "row", gap: "5px", alignItems: "center", children: I }) }) })] })] })] }) }) });
}), eo = k()({ reconnectButton: { textTransform: "none" }, closeTerminalButton: { textTransform: "none", backgroundColor: "var(--dark-grey-6)" }, invisible: { opacity: 0, visibility: "hidden", overflow: "hidden", width: 0, height: 0 }, visible: { visibility: "visible", animation: `${le} 0.3s forwards` } });
function oo(o, n) {
  if (!n) return null;
  switch (o) {
    case "unauthorized":
    case "starter":
      return { title: `You have ${Math.floor(ye(n))}mb of logs stored locally`, description: "Manage log storage in settings" };
    default:
      return null;
  }
}
function to({ userType: o, ...n }) {
  const a = N(), e = E((l) => l.history.sessionLogsStorageSize), [u] = E(he), c = { promotionLocation: "Vault Tab", promotionType: "High-Volume Unsynced Logs", trialDaysLeft: u, sideMenuTeamTrialPromoType: void 0 };
  fe(() => {
    a(ge(c));
  });
  const d = () => {
    a(me({ ...c, dismissForever: void 0 })), a(xe()), n.onClose();
  }, r = () => {
    n.onClose(), a(Ce("/logs")), a(be(c));
  };
  if (!e) return null;
  const s = oo(o, e);
  return h(S, { ...n, onClose: d, handlers: [r], onClickAway: T, actions: ["Manage logs"], children: [h("strong", { children: [s == null ? void 0 : s.title, i("br", {})] }), s == null ? void 0 : s.description] });
}
function no({ before: o, shortcut: n, after: a, shortcutsScheme: e, ...u }) {
  var s;
  const { classes: c } = so(), d = (s = e == null ? void 0 : e.shortcuts.find((l) => l.type === n)) == null ? void 0 : s.shortcut, r = i(C, { direction: "row", gap: "3px", margin: "0 5px", children: d == null ? void 0 : d.split("+").map((l) => i("div", { className: c.shortcut, children: i(b, { variant: "mediumBody1", fontSize: 11, lineHeight: "18px", children: ke(l) }) }, l)) });
  return i(S, { ...u, children: h(C, { direction: "row", children: [o, r, a] }) });
}
const so = k()({ shortcut: { color: "var(--hotkey-text-color)", background: "var(--hotkey-background-color)", borderRadius: 3, textAlign: "center", fontFamily: "Lucida Grande, CircularXX, sans-serif", padding: "0 3px", minWidth: "18px" } });
function ro({ message: o, ...n }) {
  const a = N(), e = Se(), u = () => {
    n.onClose(), a(we({ windowId: B() })), a(z({ hintMessage: "Inactive Pro Features", hintLevel: "Info", closingMethod: void 0 }));
  };
  return i(S, { ...n, onClose: u, handlers: [() => {
    Ae(e, { dialog: "ProFeatures", target: B(), props: { purchaseOpen: true, from: "Connection Flow" } }).catch(T), u();
  }], onClickAway: T, children: o });
}
const lo = D.memo(() => {
  const o = N(), n = E(He), a = E(Ee), { classes: e } = ao(), u = Ne(n, ({ createdAt: t }) => t), c = (t) => {
    o(De(t));
  }, d = E(Te), r = V($e), s = B(), l = d === s, p = r === "terminal" ? "adaptive" : "main";
  return i(Me, { in: l, children: i("div", { className: e.root, children: i(Ie, { children: u.map((t) => i(Fe, { timeout: 300, classNames: { enter: e.enter, enterActive: e.enterActive, exit: e.exit, exitActive: e.exitActive }, children: h(D.Fragment, { children: [t.variant === "alert" && i(Ve, { title: t.title, message: t.message, onClose: () => c(t.id), colorMode: p }), t.variant === "process" && i(Je, { id: t.id, message: t.message, target: t.target, onClose: () => c(t.id), createdAt: t.createdAt, timer: t.timer }), t.variant === "reconnect" && i(Ye, { id: t.id, onClose: () => c(t.id), createdAt: t.createdAt, timer: t.timer, currentWindowFocused: l }), t.variant === "message" && i(Ge, { message: t.message, type: t.type, onClose: () => c(t.id), actions: t.actions, timer: t.timer, colorMode: p }), t.variant === "shortcut" && i(no, { before: t.before, shortcut: t.shortcut, after: t.after, type: t.type, onClose: () => c(t.id), actions: t.actions, timer: t.timer, colorMode: p, shortcutsScheme: a }), t.variant === "fileTypeAssociation" && i(Ke, { association: t.association, type: t.type, onClose: () => c(t.id), actions: t.actions, timer: t.timer, colorMode: p }), t.variant === "proFeatures" && l && i(ro, { message: t.message, type: t.type, onClose: () => c(t.id), actions: t.actions, timer: t.timer, colorMode: p }), t.variant === "sessionLogsUnsynced" && s === t.windowId && i(to, { type: "regular", onClose: () => c(t.id), colorMode: p, timer: t.timer, userType: t.userType })] }) }, t.id)) }) }) });
}), ao = k()({ root: { zIndex: 1302, position: "fixed", marginLeft: "20px", bottom: "20px" }, enter: { opacity: 0 }, enterActive: { opacity: 1, transition: "opacity 300ms ease-in" }, exit: { opacity: 1 }, exitActive: { opacity: 0, transition: "opacity 300ms ease-in" } });
export {
  lo as SnackbarController
};
