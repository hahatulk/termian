/*
 * Recovered Termius UI module: FontAutocomplete-0a9583b4.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
var j = Object.defineProperty;
var _ = (e, r, s) => r in e ? j(e, r, { enumerable: true, configurable: true, writable: true, value: s }) : e[r] = s;
var g = (e, r, s) => (_(e, typeof r != "symbol" ? r + "" : r, s), s);
import { bO as N, br as V, cD as X, bt as l, e3 as I, gf as v, yz as Z, dZ as G, cu as J, hc as Q, ba as Y, r as a, zr as W, h$ as ee, zs as te, cL as se, e7 as re, bi as z, hE as L, zt as oe, gd as ae, pt as ne, gg as le, oW as ie, cX as de, b0 as ce } from "./reconnectSaga-3b11f62e.js";
const m = /* @__PURE__ */ new Map([["Error", "#F25E61"], ["Warning", "#FFCB00"], ["OK", "#81D254"], ["Info", "#2091F6"], ["Debug", "#8686FF"], ["IP address & MAC", "#D2549A"]]), F = /* @__PURE__ */ new Map([["Error", "#FF000F"], ["Warning", "#E36D00"], ["OK", "#009618"], ["Info", "#0029FF"], ["Debug", "#9E00E9"], ["IP address & MAC", "#EE0064"]]), O = /* @__PURE__ */ new Map([["Error", "#FF9698"], ["Warning", "#FFB55B"], ["OK", "#9FFFCE"], ["Info", "#56C2FF"], ["Debug", "#B4B4FF"], ["IP address & MAC", "#FF9FD4"]]), ue = /* @__PURE__ */ new Map([["Error", "#FF7679"], ["Warning", "#F2FF5B"], ["OK", "#6CFFB3"], ["Info", "#7AF7FF"], ["Debug", "#CBA1FF"], ["IP address & MAC", "#FF82EB"]]), pe = /* @__PURE__ */ new Map([["Error", "#B20004"], ["Warning", "#FFC700"], ["OK", "#39FF27"], ["Info", "#0000FF"], ["Debug", "#8000FF"], ["IP address & MAC", "#BC0093"]]), he = /* @__PURE__ */ new Map([["termius dark", m], ["homebrew", m], ["pro", m], ["solarized dark", m], ["dracula", m], ["monokai", m], ["nord dark", m], ["termius light", F], ["basic", F], ["man page", F], ["novel", F], ["solarized light", F], ["nord light", F], ["grass", O], ["red sands", O], ["ocean", ue], ["silver aerogel", pe]]);
function Ie(e = "") {
  const r = he.get(e.toLowerCase());
  return r || m;
}
function Me(e) {
  return e.localTerminalPath.path;
}
const ge = "127.0.0.1", me = "22";
class fe extends N.Component {
  constructor() {
    super(...arguments);
    g(this, "state", { address: ge, port: me, username: "", password: "", errors: { port: false, username: false, password: false } });
    g(this, "handleSubmit", (s) => {
      s.preventDefault();
      const { address: i, port: u, username: f, password: c } = this.state;
      this.props.onOk({ address: i, port: u, username: f, password: c });
    });
    g(this, "setError", (s) => {
      this.setState({ errors: { ...this.state.errors, ...s } });
    });
    g(this, "onPortChange", (s) => {
      const i = s.target.value.trim();
      this.setState({ port: i }), this.setError({ port: i.length === 0 });
    });
    g(this, "onUsernameChange", (s) => {
      const { value: i } = s.target;
      this.setState({ username: i }), this.setError({ username: i.length === 0 });
    });
    g(this, "onPasswordChange", (s) => {
      const { value: i } = s.target;
      this.setState({ password: i }), this.setError({ password: i.length === 0 });
    });
    g(this, "isDisabled", () => {
      const s = this.state.username.trim(), i = this.state.password.trim(), u = this.state.port.trim();
      return s.length === 0 || i.length === 0 || u.length === 0;
    });
  }
  render() {
    const s = this.isDisabled();
    return V(X, { title: "Enter required fields", onClose: this.props.onClose, onOk: this.handleSubmit, disabled: s, buttonColor: "accent", dark: this.props.dark, children: [l(I, { children: l(v, { label: "Address", value: this.state.address, dark: this.props.dark, fullWidth: true, disabled: true, children: l(v, { label: "Port", value: this.state.port, error: this.state.errors.port, onChange: this.onPortChange, onBlur: this.onPortChange, required: true, dark: this.props.dark, port: true, fullWidth: true }) }) }), l(I, { children: l(v, { label: "User", value: this.state.username, error: this.state.errors.username, onChange: this.onUsernameChange, onBlur: this.onUsernameChange, required: true, dark: this.props.dark, fullWidth: true, autoFocus: true }) }), l(I, { children: l(Z, { label: "Password", value: this.state.password, error: this.state.errors.password, onChange: this.onPasswordChange, onBlur: this.onPasswordChange, required: true, dark: this.props.dark, fullWidth: true }) })] });
  }
}
const ye = G.promisify(J(fe)), Be = Q((e) => e.localhost, (e) => e != null ? new Y(e) : null), Ce = (e, r) => a.createElement("svg", { width: 10, height: 2, viewBox: "0 0 10 2", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: r, ...e }, a.createElement("rect", { width: 10, height: 2, rx: 1, fill: "currentColor" })), Fe = a.forwardRef(Ce), Ae = a.memo(Fe), Re = [{ ...W.helium, variant: "helium" }, { ...W.hydrogen, variant: "hydrogen" }, { name: "Disabled" }], Se = [{ name: "Enabled" }, { name: "Disable for this session", variant: "session" }, { name: "Disable for today", variant: "day" }, { name: "Disable completely", variant: "forever" }], We = (e) => [...ee(!!e, [{ name: "Enabled", variant: te[e] }]), { name: "Disabled" }], we = a.memo(({ font: e, onSelectFont: r, isSelected: s, containerRef: i, className: u }) => {
  const { classes: f, cx: c } = be();
  return l("div", { ref: i, children: l(se, { className: c(f.root, u), endAdornment: s && l(re, { className: f.icon }), dense: true, onClick: () => r(e), style: { fontFamily: e }, children: e }) });
}), be = z()({ root: { marginRight: 4 }, icon: { width: "15px", height: "12px" } }), ve = 350, T = (e) => {
  setTimeout(e, ve + 50);
}, Le = ({ currentFont: e, onFontSubmitted: r, menuFullWidth: s = false, isMenuOnTop: i = false, colorScheme: u }) => {
  const f = oe(u), c = ae(), b = a.useRef(null), M = a.useRef(null), y = a.useRef(null), { classes: C, cx: x } = Ee(), [U, k] = a.useState(e), [q, B] = a.useState(""), [p, E] = a.useState(e), D = c.isFocused && !!c.current, A = ne.getFontFamilies(), d = A.filter((t) => t.toLowerCase().indexOf(q.toLowerCase()) > -1), H = (t) => {
    B(t), k(t), K(t);
  }, R = a.useCallback(() => {
    k(e), T(() => E(e)), T(() => {
      B("");
    });
  }, [e]);
  a.useEffect(() => {
    var t;
    D ? (t = b == null ? void 0 : b.current) == null || t.scrollIntoView({ block: "nearest" }) : R();
  }, [D, R]), a.useEffect(() => {
    k(e);
  }, [e]);
  const K = (t) => {
    const o = A.filter((n) => n.toLowerCase().indexOf(t.toLowerCase()) > -1);
    o.length && (o.find((h) => h === p) || E(o[0]));
  }, w = a.useCallback((t) => d.length ? (d.findIndex((n) => n === p) + t + d.length) % d.length : null, [d, p]), P = a.useCallback((t) => {
    const o = w(t);
    if (o != null) {
      const n = t === 1 ? M.current : y.current;
      n == null || n.scrollIntoView({ block: "nearest" }), r(d[o]);
    }
    E(o != null ? d[o] : null);
  }, [d, w, r]), S = a.useCallback(() => {
    var t;
    if (d.length) {
      const o = d.find((n) => n === p);
      o && (r(o), (t = c == null ? void 0 : c.current) == null || t.blur());
    }
  }, [d, p, r, c]);
  a.useEffect(() => {
    const t = c.current, o = (n) => {
      const h = n.key.toLowerCase();
      h.includes("esc") && t && (t.blur(), n.stopPropagation()), h.includes("arrowup") && (P(-1), n.stopPropagation()), h.includes("arrowdown") && (P(1), n.stopPropagation()), h.includes("enter") && (S(), n.stopPropagation());
    };
    return t == null || t.addEventListener("keydown", o), () => t == null ? void 0 : t.removeEventListener("keydown", o);
  }, [c, P, S]);
  const $ = a.useCallback((t, o) => {
    const n = w(-1), h = w(1);
    return t === n ? y : t === h ? M : o === p ? b : null;
  }, [w, p]);
  return l(N.Fragment, { children: V("div", { className: C.container, children: [l(v, { outlined: true, disableWrapper: true, inputRef: c.ref, value: U, onChange: (t) => H(t.target.value), className: x(C.text, { [C.themeColors]: !!u }), accentColor: ke(u) }), l(le, { isOpen: D, anchorEl: c.current, className: x(C.dropdown, f), popperClassName: C.popper, disablePortal: true, offset: [0, 4], popperFlip: true, popperPreventOverflow: { enabled: false }, fullWidth: s, placement: i ? "top-start" : "bottom-start", children: l(xe, { colorScheme: u, children: d != null && d.length ? d.map((t, o) => l(we, { className: x(f, { [C.selected]: p === t }), colorScheme: u, font: t, isSelected: p === t, onSelectFont: r, containerRef: $(o, t) }, `font-${t}`)) : l("div", { className: C.empty, children: "No results" }) }) })] }) });
};
function xe({ colorScheme: e, children: r }) {
  const s = ie({ colorScheme: e });
  return l(de, { maxHeight: "350px", padding: "5px", ref: s.ref, className: s.className, children: r });
}
const ke = (e) => {
  var s;
  const r = (s = ce(e == null ? void 0 : e.backgroundColor)) == null ? void 0 : s.isLight();
  return e ? r ? "var(--black-a05)" : "var(--white-a05)" : "var(--dark-grey-6)";
}, Ee = z()({ container: { width: "100%", position: "relative" }, text: { width: "100%" }, themeColors: { "--text-color": L.uiColor, "&:hover": { "--text-color": L.foregroundColor } }, dropdown: { "--menu-background-color": "var(--foreground)", "--menu-item-text-color": "var(--main)", "--menu-item-text-hover-color": "var(--main)", "--menu-item-svg-hover-color": "var(--main)", background: "var(--menu-background-color)", border: "1px solid", borderColor: "transparent", borderRadius: 10, overflow: "unset" }, popper: { borderRadius: 10, padding: 0 }, empty: { padding: "0 14px", minWidth: "150px" }, selected: { color: "var(--menu-item-text-hover-color)", "& svg": { color: "var(--menu-item-svg-hover-color)" } } });
export {
  Le as F,
  Ae as M,
  Se as a,
  We as b,
  Re as c,
  Ie as g,
  Be as l,
  Me as p,
  ye as r
};
