/*
 * Recovered Termius UI module: index-7ecae23c.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
var ee = Object.defineProperty;
var te = (t, a, o) => a in t ? ee(t, a, { enumerable: true, configurable: true, writable: true, value: o }) : t[a] = o;
var l = (t, a, o) => (te(t, typeof a != "symbol" ? a + "" : a, o), o);
import { bi as u, zr as oe, br as s, bs as k, bt as e, bu as f, cH as q, cY as L, cL as j, e7 as ne, pt as M, r as g, cy as y, cv as E, gf as H, pu as ae, ft as re, ms as se, s9 as le, bq as Y, bj as Q, qh as ie, qg as B, BH as ce, bP as O, bQ as de, BI as he, p8 as pe, ej as ge, cA as me, cB as U, X as v, BJ as ue, BK as be, BL as R, BM as fe, BN as Se, b3 as xe, cz as z, iq as ye, BO as Ce, BP as ve, jV as Te, hE as F, bO as _, BQ as ke, pS as we, AP as Ie, pM as Pe, py as Be, BR as Ne, BS as Ae, BT as Ee, BU as Le, BV as He, BW as Oe, BX as De, BY as Me, ay as N, BZ as Re, gY as ze, B_ as Fe, B$ as _e, C0 as $e, C1 as Ke, C2 as Ve, C3 as We, C4 as qe, C5 as je, nX as $, nY as K, oM as Ye, ob as V, nQ as W, C6 as Qe, C7 as Ue, C8 as Xe, C9 as Ge, q2 as Je, qf as Ze, q3 as et, q7 as tt, o9 as ot, o1 as nt, q4 as at, ow as rt, ox as st, og as lt, q5 as it, uD as ct, ot as dt, oK as ht, nN as pt, gn as gt } from "./reconnectSaga-3b11f62e.js";
import { S as b } from "./SettingsSwitch-d35e9651.js";
import { F as X } from "./FiltersButton-2e4a50c6.js";
import { b as mt, c as ut, M as bt, g as ft, p as St, r as xt, F as yt, l as Ct } from "./FontAutocomplete-0a9583b4.js";
import { S as w } from "./SettingsText-0178ea31.js";
import { a as P, S } from "./SettingsSection-bc44d6bf.js";
import { S as vt } from "./SettingsTabContent-ce30872f.js";
const Tt = ({ activeVariant: t, experimentTreatment: a = "disabled", onChange: o }) => {
  const { classes: n, cx: r } = kt(), i = t ? oe[t].name : void 0, c = a !== "disabled", h = c ? mt(a) : ut;
  return s(k, { direction: "row", className: n.container, children: [s(k, { direction: "row", gap: 5, alignItems: "center", children: [e(f, { className: n.text, color: "primary", children: "Autocomplete" }), e(f, { fontSize: 10, bold: true, uppercase: true, color: "secondary", className: n.badge, children: "Beta" })] }), e(q, { toggler: (d) => s(X, { className: n.text, children: [t ? e(k, { direction: "row", justifyContent: "space-between", alignItems: "center", gap: 5, children: c ? "Enabled" : i }) : "Disabled", e(L, { className: r(n.arrowIcon, { [n.open]: d.isOpen }) })] }), children: ({ closeMenu: d }) => h.map(({ name: m, variant: p }) => e(j, { sx: { width: 200 }, dense: true, onClick: () => {
    o(p), d();
  }, children: s(k, { direction: "row", justifyContent: "space-between", alignItems: "center", width: "100%", children: [e(k, { direction: "row", justifyContent: "space-between", alignItems: "center", gap: 10, children: e(f, { children: m }) }), p === t && e(ne, {})] }) }, m)) })] });
}, kt = u()({ container: { display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%", marginTop: "15px" }, text: { fontSize: "12px", fontWeight: 500 }, arrowIcon: { width: "15px", height: "6px", paddingLeft: "5px", fill: "var(--filters-header-color)", marginTop: "2px" }, open: { transform: "rotate(180deg)", paddingLeft: "0px", paddingRight: "5px" }, badge: { borderRadius: 3.75, padding: "1.5px 5px", lineHeight: "125%", border: "0.75px solid var(--secondary)", "button:hover &": { borderColor: "currentColor", color: "currentColor" } } }), wt = ({ onChange: t, currentFontSize: a }) => {
  const { classes: o } = It(), n = M.getMinFontSize(), r = M.getMaxFontSize(), [i, c] = g.useState(`${a}`);
  g.useEffect(() => {
    c(`${a}`);
  }, [a]);
  const h = (p) => {
    const I = p.replace(/[^\d]/g, ""), C = parseInt(I, 10);
    if (Number.isNaN(C)) {
      c(I);
      return;
    }
    if (C < n) {
      c(`${C}`);
      return;
    }
    const T = C > r ? r : C;
    c(`${T}`), T !== a && t(T);
  }, d = () => {
    if (a > n) {
      const p = a - 1;
      t(p), c(`${p}`);
    }
  }, m = () => {
    if (a < r) {
      const p = a + 1;
      t(p), c(`${p}`);
    }
  };
  return s("div", { className: o.container, children: [e(y, { right: 10, children: e(f, { fontSize: 14, lineHeight: 1.5, className: o.title, children: "Text Size" }) }), s("div", { className: o.content, children: [e(E, { onClick: d, className: o.buttonItem, disabled: a === n, children: e(bt, {}) }), e(H, { outlined: true, disableWrapper: true, value: i, onChange: (p) => h(p.target.value), className: o.fontInput, isTextOnCenter: true }), e(E, { onClick: m, className: o.buttonItem, disabled: a === r, children: e(ae, {}) })] })] });
}, It = u()({ container: { width: "100%", display: "flex", justifyContent: "space-between", alignItems: "center" }, content: { display: "flex" }, buttonItem: { height: "36px", width: "36px", fontWeight: 700, fontSize: "16px", backgroundColor: "var(--dark-grey-6)", borderRadius: "6px", ".termius-dark-theme &": { backgroundColor: "var(--dark-grey-4)" } }, fontInput: { width: "51px", margin: "0 5px" }, title: { color: "var(--dark-grey-1)", ".termius-dark-theme &": { color: "white" } } }), Pt = 2 ** 31 - 1;
function Bt(t) {
  const a = Number.parseInt(t, 10);
  return Number.isNaN(a) ? 0 : le(a, 0, Pt);
}
function Nt({ onRemoveKeepAliveInterval: t, onChangeKeepAliveInterval: a, value: o }) {
  const { classes: n } = At(), { register: r, watch: i, setValue: c } = re({ defaultValues: { value: o ?? "0" } }), h = i("value");
  g.useEffect(() => {
    const m = Bt(h), p = String(m);
    m === 0 ? t() : a(p), h !== p && c("value", p);
  }, [h, a, t, c]);
  const d = Number(h);
  return s("div", { className: n.wrapper, children: [e(P, { children: "Keepalive Interval" }), e(w, { children: "How often (in seconds) to send SSH-level keepalive packets to the server. Set to 0 to disable." }), e(y, { top: 10 }), e(se, { InputProps: r("value"), variant: "standard", label: "Interval", id: "keepAliveInterval", fullWidth: true, endAdornment: !!d && s("div", { className: n.endAdornment, children: [e("div", { className: n.hidden, children: d }), s("div", { className: n.postfix, children: ["second", d > 1 ? "s" : ""] })] }) })] });
}
const At = u()({ wrapper: { "--text-color": "var(--dark-grey-1)", ".termius-dark-theme &": { "--text-color": "white" } }, endAdornment: { position: "absolute", left: "0", pointerEvents: "none", display: "flex" }, hidden: { opacity: "0" }, postfix: { marginLeft: "3px" } }), D = (t) => {
  const { children: a } = t, { classes: o } = Et();
  return e(f, { className: o.title, children: a });
}, Et = u()({ title: { fontSize: 14, fontWeight: 700, lineHeight: "24px", wordBreak: "break-word", display: "flex", alignItems: "center", justifyContent: "flex-start", color: "var(--dark-grey-1)", ".termius-dark-theme &": { color: "white" } } }), G = (t) => {
  const { children: a } = t, { classes: o } = Lt();
  return e(f, { className: o.title, children: a });
}, Lt = u()({ title: { fontSize: 12, fontWeight: 500, lineHeight: "18px", color: "var(--dark-grey-1)", ".termius-dark-theme &": { color: "white" } } }), Ht = (t) => {
  const { children: a } = t, { classes: o } = Ot();
  return e(f, { className: o.text, children: a });
}, Ot = u()({ text: { fontSize: 12, color: "var(--light-grey-1)", ".termius-dark-theme &": { color: "var(--light-grey-3)" } } }), Dt = (t) => {
  const { name: a, color: o, onColorItemPressed: n } = t, { classes: r } = Mt();
  return s("div", { className: r.container, children: [e(f, { className: r.text, fontSize: 12, color: "primary", children: a }), e("div", { style: { backgroundColor: o }, className: r.colorRectangle, onClick: () => n() })] });
}, Mt = u()({ container: { display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%", marginTop: "15px" }, text: { fontSize: "12px", fontWeight: 500 }, colorRectangle: { width: "50px", height: "25px", display: "block", borderRadius: "6px" } }), Rt = (t) => {
  const { checked: a, theme: o, onOpenProductBoard: n, onChange: r, onColorItemPressed: i } = t, c = ft(o || ""), h = Array.from(c, ([d, m]) => e(Dt, { name: d, color: m, onColorItemPressed: i }, `highlighting-group-${d}`));
  return s("div", { children: [e(b, { checked: a, topMargin: 0, label: e(D, { children: "Keyword highlighting" }), onChange: r }), e(y, { top: 20, children: h }), e(w, { children: "Would you like to add custom rules to highlight output in the terminal?" }), e(y, { top: 10, children: e(E, { size: "medium", color: "accent", type: "submit", onClick: () => n(), children: "Let us know" }) })] });
}, zt = () => {
  const t = Y(), a = Q(ie), o = a === B ? 0 : a, [n, r] = g.useState(o), i = g.useCallback((h) => {
    const d = Math.abs(Number.parseInt(h.target.value, 10));
    d > B ? r(0) : r(d || 0);
  }, []), c = g.useCallback(() => {
    t(ce(n || B));
  }, [t, n]);
  return s("div", { children: [e(P, { children: "Scrollback" }), e(y, { top: 10 }), e(w, { children: "Limit number of terminal rows. Set to 0 to maximum limit size." }), e(H, { label: "Number of rows", type: "number", onChange: i, onBlur: c, value: n, required: true, fullWidth: true })] });
}, Ft = ["xterm-256color", "xterm-16color", "xterm"], _t = (t) => {
  const { selected: a, onChange: o } = t, { classes: n } = $t();
  return s("div", { className: n.container, children: [e(f, { className: n.text, fontSize: 12, color: "primary", children: "Terminal emulation type" }), e(q, { toggler: (r) => s(X, { className: n.text, children: [a, e(L, { className: O(n.arrowIcon, { [n.open]: r.isOpen }) })] }), children: ({ closeMenu: r }) => Ft.map((i) => e(j, { dense: true, onClick: () => {
    o(i), r();
  }, children: i }, i)) })] });
}, $t = u()({ container: { display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%", marginTop: "15px" }, text: { fontSize: "12px", fontWeight: 500 }, arrowIcon: { width: "15px", height: "6px", paddingLeft: "5px", fill: "var(--filters-header-color)", marginTop: "2px" }, open: { transform: "rotate(180deg)", paddingLeft: "0px", paddingRight: "5px" } });
function Kt(t) {
  return t.localTerminalPath.customPaths;
}
const Vt = g.forwardRef((t, a) => e(de, { ref: a, fullWidth: true, variant: "standard", InputProps: { inputComponent: jt, inputProps: { className: t.selectProps.classes.input, inputRef: t.innerRef, children: t.children, ...t.innerProps } }, ...t.selectProps.textFieldProps })), Wt = g.forwardRef((t, a) => e(he, { square: true, ref: a, className: t.selectProps.classes.paper, ...t.innerProps, children: t.children })), qt = g.forwardRef((t, a) => s("div", { ref: a, className: t.selectProps.classes.option, ...t.innerProps, children: [e(f, { fontSize: 14, className: t.selectProps.classes.optionLabel, ellipsis: true, children: t.children }), t.data.custom && e(pe, { "aria-label": "Delete", className: t.selectProps.classes.optionDeleteButton, onClick: (o) => t.selectProps.onDelete(o, t.data), children: e(ge, {}) })] })), jt = ({ inputRef: t, ...a }) => e("div", { ref: t, ...a }), Yt = { Menu: Wt, Option: qt, Control: Vt, IndicatorSeparator: null };
class Qt extends g.Component {
  constructor() {
    super();
    l(this, "detectShells", async () => {
      try {
        const o = await ue();
        this.setState({ detectedShells: A(o, false) });
      } catch {
      }
    });
    l(this, "handleSelect", (o) => {
      o != null && (o.__isNew__ && this.props.dispatch(be(o.label)), this.props.dispatch(R(o)));
    });
    l(this, "handleDelete", (o, n) => {
      o.stopPropagation(), this.props.dispatch(fe(n.label)), n.label === this.props.path && this.props.dispatch(R(null));
    });
    this.state = { detectedShells: A([v().defaultLocalTerminalShell], false) };
  }
  componentDidMount() {
    this.detectShells();
  }
  render() {
    const o = [...this.state.detectedShells, ...A(this.props.customPaths, true)], n = { input: (r) => ({ ...r, color: "var(--text-primary)", fontSize: 14, margin: 0, "& input": { font: "inherit" } }), container: (r) => ({ ...r, marginTop: "20px" }), valueContainer: () => ({ padding: 0, flexGrow: 1, overflow: "hidden", position: "relative" }), dropdownIndicator: () => ({ padding: 0, color: z.colors.secondary.darker, transform: "rotate(180deg)" }), singleValue: (r) => ({ ...r, color: z.colors.secondary.dark, margin: 0, fontSize: 14, paddingRight: 10, overflowX: "auto", textOverflow: "unset", position: "absolute", "&::-webkit-scrollbar": { display: "none" } }) };
    return s("div", { className: this.props.classes.wrapper, children: [e(P, { children: "Local Terminal Path" }), e(Se, { onDelete: this.handleDelete, classes: this.props.classes, value: Jt(this.props.path || v().defaultLocalTerminalShell), onChange: this.handleSelect, options: o, getOptionLabel: (r) => r.label, getOptionValue: (r) => r.label, components: Yt, styles: n })] });
  }
}
const Ut = { root: {}, wrapper: {}, paper: { "--menu-bg-color": "var(--light-grey-7)", "--menu-border-color": "var(--light-grey-transparent-10)", ".termius-dark-theme &, &.termius-dark-theme": { "--menu-bg-color": "var(--dark-grey-3)", "--menu-border-color": "var(--dark-grey-transparent-10)" }, backgroundColor: "var(--menu-bg-color)", border: "1px solid", borderColor: "var(--menu-border-color)", boxShadow: "0px 15px 30px 0px rgba(0, 0, 0, 0.15)", overflow: "visible", borderRadius: "15px", padding: 0, position: "absolute", bottom: "24px", zIndex: 2, right: 0, maxWidth: "100%", minWidth: 200, "& > div": { padding: "8px 0 8px 8px", overflowY: "scroll", "&::-webkit-scrollbar-track": { display: "flex", margin: 8 } } }, input: { display: "flex", padding: 0, height: "auto" }, option: { "--hover-color": "var(--light-grey-transparent-10)", "--text-color": "var(--dark-grey-1)", ".termius-dark-theme &": { "--hover-color": "var(--dark-grey-transparent-10)", "--text-color": "var(--white)" }, padding: "5px 10px", borderRadius: "10px", height: 40, cursor: "default", color: "var(--text-color)", display: "flex", justifyContent: "space-between", alignItems: "center", "&:hover": { backgroundColor: "var(--hover-color)" } }, optionLabel: { flex: 1 }, selected: { backgroundColor: "var(--background)" }, optionDeleteButton: { "--svg-color": "var(--light-grey-1)", ".termius-dark-theme &": { "--svg-color": "var(--dark-grey-7)" }, borderRadius: "6px", width: 23, height: 23, "& svg": { color: "var(--svg-color)", fontSize: 18 }, "&:hover": { backgroundColor: "var(--light-grey-transparent)", "& svg": { color: "var(--red)" }, ".termius-dark-theme &": { backgroundColor: "var(--dark-grey-transparent)" } } } }, Xt = (t) => ({ path: St(t), customPaths: Kt(t) }), Gt = me(U(Xt)(Qt), Ut, { name: "ConnectedLocalTerminalPath" });
function Jt(t) {
  return { label: t };
}
function A(t, a) {
  return t.filter(xe).map((o) => ({ label: o, custom: a }));
}
const Zt = (t) => e("span", { className: "text-bright", children: e(G, { children: t }) }), eo = `# get the shell name to choose the script for detecting OS
  echo $SHELL

# for routeros
  :put [/system resource get platform]

# for fish shell
  if set name (uname) = "Linux"
      cat /etc/*release
  else
      uname
  end

# for others
  HISTFILE=;
  SA_OS_TYPE="Linux"
  REAL_OS_NAME=\`uname\`
  if [ "$REAL_OS_NAME" != "$SA_OS_TYPE" ] ;
  then
  echo \`uname\`
  else
  DISTRIB_ID=\\"\`cat /etc/*release\`\\"
  echo $DISTRIB_ID;
  fi;
  exit;`, to = () => {
  const { classes: t } = oo(), [a, o] = g.useState(false), n = Y(), r = Q(ye), i = g.useCallback((h, d) => {
    n(d ? Ce() : ve());
  }, [n]), c = g.useCallback(() => {
    o((h) => !h);
  }, []);
  return s(g.Fragment, { children: [e(P, { children: "Detect Host Operating System" }), e(w, { children: "This option will execute a script when connecting to a host for the first time to detect its operating system. This will allow Termius to show a corresponding OS icon in the list of hosts." }), e(w, { children: "The script will be executed only if server didn't return anything about its OS." }), e(y, { bottom: 15, children: e(b, { checked: r, label: Zt("Detect OS"), onChange: i }) }), s("div", { className: t.accordion, children: [s("button", { className: t.header, onClick: c, type: "button", children: [e(D, { children: "Detect OS Script" }), e(L, { className: O(t.icon, { [t.expandedIcon]: a }) })] }), e(Te, { in: a, timeout: "auto", children: e(H, { label: "Script", multiline: true, fullWidth: true, value: eo, outlined: true, disabled: true, className: t.scripts }) })] })] });
}, oo = u()({ accordion: { borderRadius: 6, padding: "0 10px", backgroundColor: "var(--light-grey-6)", ".termius-dark-theme &": { backgroundColor: "var(--dark-grey-4)" } }, icon: { fill: "var(--light-grey-2)", transform: "rotate(-90deg)", transition: "transform 0.2s" }, expandedIcon: { transform: "rotate(0)" }, header: { width: "100%", border: "none", backgroundColor: "transparent", display: "flex", justifyContent: "space-between", alignItems: "center", padding: "10px 0" }, scripts: { padding: "5px 0 10px 0", "--form-disabled-input-color": "var(--light-grey-1)", ".termius-dark-theme &": { "--form-disabled-input-color": "var(--dark-grey-7)" }, "& textarea": { fontFamily: "monospace" }, "& textarea::-webkit-scrollbar": { display: "none" } } });
function no({ title: t, className: a, withOpacity: o = false }) {
  const { classes: n, cx: r } = ao();
  return e("span", { className: r(n.betaBadge, { [n.basicColors]: !o, [n.opacityColors]: o }, a), children: t });
}
const ao = u()({ betaBadge: { borderRadius: 4, textTransform: "uppercase", marginLeft: 10, fontSize: "0.714em", fontWeight: 700, padding: "3px 8px" }, basicColors: { color: "var(--dark-grey-1)", backgroundColor: "var(--light-grey-5)", ".termius-dark-theme &": { color: "var(--white)", backgroundColor: "var(--dark-grey-5)" } }, opacityColors: { backgroundColor: "transparent", color: F.uiColor, border: "1px solid", borderColor: F.uiColorA10 } }), x = (t) => {
  const { title: a, beta: o } = t, { classes: n } = ro();
  return s("span", { className: O("text-bright", n.container), children: [e(G, { children: a }), o && e(no, { title: "Beta" })] });
}, ro = u()({ container: { display: "flex", flexDirection: "row" } });
class so extends _.Component {
  constructor() {
    super(...arguments);
    l(this, "onChangeColorScheme", (o) => {
      this.props.dispatch(we(o));
      const n = Ie(o);
      n && this.props.dispatch(Pe({ terminalTheme: n, terminalThemeSource: "App Preferences", globalTerminalTheme: this.props.getColorScheme(o).name, isWorkspace: "no", workspaceTemplateID: void 0 }));
    });
    l(this, "onChangeFontFamily", (o) => {
      this.props.dispatch(Be({ terminalFont: this.props.fontFamily, terminalFontSource: "Settings", terminalFontSelected: o })), this.props.dispatch(Ne(o));
    });
    l(this, "onChangeFontSize", (o) => {
      this.props.dispatch(Ae(o));
    });
    l(this, "onCopyPasteToggle", (o, n) => {
      const { dispatch: r } = this.props;
      r(n ? Ee() : Le());
    });
    l(this, "onBellSoundToggle", (o, n) => {
      const { dispatch: r } = this.props;
      r(n ? He() : Oe());
    });
    l(this, "onOptionIsMetaToggle", (o, n) => {
      const { dispatch: r } = this.props;
      r(n ? De() : Me());
    });
    l(this, "onChangeKeepAliveInterval", (o) => {
      N.setKeepAliveInterval(o);
    });
    l(this, "onChangePQCKexEnabled", (o, n) => {
      this.props.dispatch(Re(n));
    });
    l(this, "onChangeLocalHost", async (o, n) => {
      const { dispatch: r } = this.props;
      if (n) {
        const i = await xt({ dark: true });
        i && r(ze(i));
      } else r(Fe());
    });
    l(this, "onChangeBrightenBoldText", async (o, n) => {
      const { dispatch: r } = this.props;
      r(n ? _e() : $e());
    });
    l(this, "onChangeTerminalEmulationType", (o) => {
      this.props.dispatch(Ke(o));
    });
    l(this, "onChangeKeywordsHighlighting", async (o, n) => {
      const { dispatch: r } = this.props;
      r(Ve(n)), r(We({ keywordHighlighting: n ? "enabled" : "disabled", keywordHighlightingEnabled: n ? "yes" : "no" }));
    });
    l(this, "openKeywordHighlightingProductBoardPage", () => {
      const { dispatch: o, onOpenProductBoard: n } = this.props;
      n("keywords highlighting"), o(qe());
    });
    l(this, "onKeywordsHighlightingColorItemPressed", () => {
      const { dispatch: o } = this.props;
      o(je());
    });
    l(this, "onRemoveKeepAliveInterval", () => {
      N.removeKeepAliveInterval();
    });
    l(this, "onAutocompleteSelect", (o) => {
      const { dispatch: n } = this.props;
      o ? (n($(K.ENABLED)), n(Ye(o)), n(V({ autocompleteVersion: W[o], autocompleteState: "enabled", connectionID: null }))) : (n($(K.DISABLED)), n(V({ autocompleteVersion: W[this.props.autocompleteVariant], autocompleteState: "disabled", connectionID: null })));
    });
    l(this, "onAutoreconnectToggle", (o, n) => {
      const { dispatch: r } = this.props;
      r(n ? Qe() : Ue());
    });
    l(this, "toggleShellHistoryImport", (o, n) => {
      this.props.dispatch(Xe(n));
    });
  }
  componentDidMount() {
    v().isSandbox && this.props.dispatch(ke());
  }
  render() {
    const { rightClickToPaste: o, bellSound: n, optionIsMeta: r, fontFamily: i, fontSize: c, currentColorSchemeName: h, brightenBoldText: d, terminalEmulationType: m, enabledAutocomplete: p, enableShellHistoryImport: I, keywordsHighlighting: C, enabledAutoreconnect: T, autocompleteVariant: J, experimentTreatment: Z } = this.props;
    return s(vt, { children: [s(S, { children: [e(D, { children: "Terminal settings" }), e(Tt, { activeVariant: p ? J : void 0, experimentTreatment: Z, onChange: this.onAutocompleteSelect }), e(b, { checked: T, label: e(x, { title: "Autoreconnect" }), onChange: this.onAutoreconnectToggle }), e(b, { checked: I, label: e(x, { title: "Import shell history" }), onChange: this.toggleShellHistoryImport }), e(b, { checked: o, label: e(x, { title: "Select text to copy & Right click to paste" }), onChange: this.onCopyPasteToggle }), e(b, { checked: n, label: e(x, { title: "Bell sound" }), onChange: this.onBellSoundToggle }), v().platform === "mac" && e(b, { checked: r, label: e(x, { title: "Use Option as Meta key" }), onChange: this.onOptionIsMetaToggle }), v().isSandbox && this.props.localhost && s(_.Fragment, { children: [e(y, { bottom: 5, children: e(b, { checked: !!this.props.localhost, label: e(x, { title: "Allow local SSH/SFTP connections" }), onChange: this.onChangeLocalHost }) }), e(Ht, { children: "Affects this device only, saved credentials will not be synced" })] }), e(b, { checked: d, label: e(x, { title: "Use bright colours for bold text" }), onChange: this.onChangeBrightenBoldText }), e(_t, { selected: m, onChange: this.onChangeTerminalEmulationType })] }), s(S, { children: [e(yt, { currentFont: i, onFontSubmitted: this.onChangeFontFamily, isMenuOnTop: true }), e(y, { top: 15, children: e(wt, { onChange: this.onChangeFontSize, currentFontSize: c }) })] }), e(S, { children: e(Ge, { title: "Terminal theme", selectedScheme: h, onChangeScheme: this.onChangeColorScheme, source: "App Preferences" }) }), e(S, { children: e(Rt, { checked: C, theme: h, onOpenProductBoard: this.openKeywordHighlightingProductBoardPage, onChange: this.onChangeKeywordsHighlighting, onColorItemPressed: this.onKeywordsHighlightingColorItemPressed }) }), e(S, { children: e(to, {}) }), !v().isSandbox && e(S, { children: e(Gt, {}) }), e(S, { children: e(Nt, { value: N.getKeepAliveInterval(), onChangeKeepAliveInterval: this.onChangeKeepAliveInterval, onRemoveKeepAliveInterval: this.onRemoveKeepAliveInterval }) }), e(S, { children: e(zt, {}) }), e(S, { children: e(b, { topMargin: 0, checked: this.props.pqcKexEnabled, label: e(x, { title: "Post-Quantum Key Exchange" }), description: "Turn off if you're experiencing issues with legacy devices", onChange: this.onChangePQCKexEnabled }) })] });
  }
}
const lo = (t) => ({ rightClickToPaste: Je(t), bellSound: Ze(t), optionIsMeta: et(t), localhost: Ct(t), currentColorSchemeName: tt(t), fontFamily: ot(t), fontSize: nt(t), brightenBoldText: at(t), terminalEmulationType: rt(t), enabledAutocomplete: st(t), enableShellHistoryImport: lt(t), keywordsHighlighting: it(t), enabledAutoreconnect: ct(t), getColorScheme: dt(t), autocompleteVariant: ht(t), experimentTreatment: pt(t), pqcKexEnabled: gt(t) }), fo = U(lo)(so);
export {
  fo as default
};
