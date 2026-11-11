/*
 * Recovered Termius UI module: index-347b6706.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { Cb as j, Cc as w, aa as G, g2 as B, X as W, Cd as A, Ce as Q, Cf as Z, a6 as $, Cg as ee, bi as P, r as p, eI as te, bt as r, cH as se, en as F, cY as ne, bP as K, cL as oe, ej as ce, p9 as z, q1 as ae, br as N, bO as _, cx as re, bu as E, fV as ie, Ch as L, Ci as le, aX as ue, an as he, ii as de, Cj as me, dI as pe, Ck as ge, b6 as R, cl as Se, fL as fe, bq as ye, Cl as be, bA as q } from "./reconnectSaga-3b11f62e.js";
import { S as Ce } from "./SettingsTabContent-ce30872f.js";
import { M as ke, K as ve } from "./KeyboardKey-4328b011.js";
function xe() {
  try {
    if (!!localStorage.getItem(j.commandPalette)) return;
    const e = localStorage.getItem(w.schemes);
    if (e == null) return;
    const t = JSON.parse(e), o = we(t);
    localStorage.setItem(j.commandPalette, JSON.stringify(true)), localStorage.setItem(w.schemes, JSON.stringify(o));
  } catch (n) {
    G.exception("Failed to migrate command palette shortcuts", n);
  }
}
function we(n) {
  const e = /* @__PURE__ */ new Set(["Mac hotkeys", "PC hotkeys"]), t = /* @__PURE__ */ new Set(["cmd+k", "cmd+j", "cmd+t"]), o = /* @__PURE__ */ new Set(["ctrl+k", "ctrl+j", "ctrl+t"]), c = /* @__PURE__ */ new Set(["commandPalette", "jumpTo", "newTab"]), s = { commandPalette: (a) => `${a}+K`, jumpTo: (a) => `${a}+J`, newTab: (a) => `${a}+T` };
  return B(n, (a) => {
    var h, m;
    const d = Object.entries(a).filter(([b]) => !e.has(b));
    for (const [b, { shortcuts: v, mod: x }] of d) for (const y of v) {
      const C = x.toLowerCase() === "cmd" && t.has(((h = y.shortcut) == null ? void 0 : h.toLowerCase()) || ""), k = x.toLowerCase() === "ctrl" && o.has(((m = y.shortcut) == null ? void 0 : m.toLowerCase()) || ""), f = C || k;
      c.has(y.type) ? y.shortcut == null && (y.shortcut = s[y.type](x)) : f && (y.shortcut = null);
    }
  });
}
class Ne {
  constructor() {
    this.migrateHotkeysSpecialSymbolsToNames(), xe();
  }
  getShortcutSchemes() {
    return JSON.parse(localStorage.getItem(w.schemes) ?? "null") ?? {};
  }
  setShortcutSchemes(e) {
    localStorage.setItem(w.schemes, JSON.stringify(e));
  }
  getCurrentScheme() {
    if (this.getShortcutsDisabled()) return { type: "disabled" };
    const e = localStorage.getItem(w.currentScheme);
    return e ? { type: "enabled", value: e } : { type: "not set" };
  }
  setCurrentScheme(e) {
    this.setShortcutsDisabled(false), localStorage.setItem(w.currentScheme, e);
  }
  disableCurrentScheme() {
    this.setShortcutsDisabled(true);
  }
  getShortcutsDisabled() {
    return JSON.parse(localStorage.getItem(w.disabled) ?? "false");
  }
  setShortcutsDisabled(e) {
    localStorage.setItem(w.disabled, JSON.stringify(e));
  }
  migrateHotkeysSpecialSymbolsToNames() {
    if (W().platform !== "mac" || localStorage.getItem(j.hotkeysSpecialSymbolsToNames)) return;
    const e = localStorage.getItem(w.schemes);
    if (e) {
      const t = e.replace(/⌘/gi, A.metaKey.mac).replace(/⌃/gi, A.ctrlKey.mac).replace(/⌥/gi, A.altKey.mac).replace(/⇧/gi, A.shiftKey.mac);
      localStorage.setItem(w.schemes, t);
    }
    localStorage.setItem(j.hotkeysSpecialSymbolsToNames, JSON.stringify(true));
  }
  static clearStorageForTests() {
    localStorage.clear();
  }
}
class Ie {
  constructor(e = W().platform, t = new Ne()) {
    this.platform = e, this.storage = t, this.storage.setShortcutSchemes(this.shortcutSchemes);
  }
  get defaultPresets() {
    return Q(this.platform);
  }
  get defaultSchemesNames() {
    return [...Object.keys(this.defaultPresets)];
  }
  get shortcutSchemes() {
    return Z(this.storage.getShortcutSchemes(), this.defaultPresets);
  }
  get currentScheme() {
    const e = this.defaultSchemesNames[0], t = this.storage.getCurrentScheme() ?? e, { shortcutSchemes: o } = this;
    switch (t.type) {
      case "disabled":
        return;
      case "not set":
        return { name: e, value: o[e] };
      case "enabled": {
        const { value: c } = t;
        return { name: c, value: o[c] ?? o[e] };
      }
      default:
        $(false, "unreachable");
        return;
    }
  }
  selectScheme(e) {
    const { shortcutSchemes: t } = this;
    e != null && t[e] == null || (e != null ? this.storage.setCurrentScheme(e) : this.storage.disableCurrentScheme());
  }
  renameScheme(e, t) {
    var c, s;
    const { shortcutSchemes: o } = this;
    (c = o[e]) != null && c.editable && (o[t] = o[e], delete o[e], ((s = this.currentScheme) == null ? void 0 : s.name) === e && this.storage.setCurrentScheme(t), this.storage.setShortcutSchemes(o));
  }
  deleteScheme(e) {
    var o, c;
    const { shortcutSchemes: t } = this;
    (o = t[e]) != null && o.editable && (delete t[e], ((c = this.currentScheme) == null ? void 0 : c.name) === e && this.storage.setCurrentScheme(this.defaultSchemesNames[0]), this.storage.setShortcutSchemes(t));
  }
  changeShortcutByType({ shortcut: e, type: t }) {
    ee(t) && this.modifyCurrentScheme(B((o) => {
      if (e != null) {
        const s = o.shortcuts.find(({ shortcut: a }) => a === e);
        s != null && (s.shortcut = null);
      }
      const c = o.shortcuts.find(({ type: s }) => t === s);
      c != null && (c.shortcut = e);
    }));
  }
  changeTabsModifier(e) {
    this.modifyCurrentScheme(B((t) => {
      t.mod = e;
    }));
  }
  changePanesModifier(e) {
    this.modifyCurrentScheme(B((t) => {
      t.paneMod = e;
    }));
  }
  modifyCurrentScheme(e) {
    const { currentScheme: t, shortcutSchemes: o } = this;
    if (t == null) return;
    const c = this.getSchemeName(t.name), s = Me(t.value, t.name);
    o[c] = e(s), this.storage.setCurrentScheme(c), this.storage.setShortcutSchemes(o);
  }
  getSchemeName(e) {
    const { shortcutSchemes: t } = this;
    if (t[e].editable) return e;
    const o = `${e} copy`;
    let c = o, s = 0;
    for (; c in t; ) c = `${o} ${s}`, s += 1;
    return c;
  }
}
function Me(n, e) {
  return B(n, (t) => {
    n.editable || (t.editable = true, t.parentScheme = e);
  });
}
const Pe = (n) => {
  const { schemes: e, onChange: t, onDeleteScheme: o } = n, c = [{ name: "Disabled" }, ...e], { classes: s } = De(), a = p.useCallback(async (d, h) => {
    d.stopPropagation();
    try {
      await te({ ...{ title: "Remove shortcuts scheme", text: "Are you sure you want to delete shortcuts scheme?", confirmButtonTitle: "Remove" }, buttonColor: "error" }), o(h.name);
    } catch {
    }
  }, [o]);
  return r(se, { toggler: ({ isOpen: d }) => r(F, { children: r(ne, { className: K({ [s.arrowIcon]: true, [s.open]: d }) }) }), children: ({ closeMenu: d }) => c.map((h) => r(oe, { dense: true, onClick: () => {
    t(h.name), d();
  }, endAdornment: h.editable ? r("div", { className: s.schemeActions, children: r(F, { color: "error", onMouseDown: (m) => m.stopPropagation(), onClick: (m) => {
    a(m, h), d();
  }, children: r(ce, { htmlColor: "inherit" }) }) }) : null, children: h.name }, h.name)) });
}, De = P()({ arrowIcon: { width: 15, height: 6, fill: "var(--light-grey-2)" }, open: { transform: "rotate(180deg)" }, schemeActions: { display: "none", marginLeft: "10px", "& svg": { fontSize: 18 } } }), Te = (n) => {
  const { isEditing: e, keys: t, keydown: o, keyup: c, onFocus: s } = n, a = p.useRef(null), { classes: d } = Be();
  return p.useLayoutEffect(() => {
    const { current: h } = a;
    if (h != null) return h.addEventListener("keydown", o, true), h.addEventListener("keyup", c), /* termian:capture-clipboard-shortcuts */
    (() => {
      const f = (l) => {
        if (typeof process === "undefined" || process.platform !== "win32") return;
        const g = { paste: "V", copy: "C", cut: "X" }[l.type];
        if (!g) return;
        l.preventDefault(), l.stopPropagation(), o({ keyCode: g.charCodeAt(0), code: "Key" + g, ctrlKey: true, altKey: false, metaKey: false, shiftKey: false, preventDefault: () => {
        }, stopPropagation: () => {
        }, currentTarget: h });
      };
      h.addEventListener("paste", f, true), h.addEventListener("copy", f, true), h.addEventListener("cut", f, true);
      return () => {
        h.removeEventListener("keydown", o, true), h.removeEventListener("keyup", c), h.removeEventListener("paste", f, true), h.removeEventListener("copy", f, true), h.removeEventListener("cut", f, true);
      };
    })();
  }, [o, c]), e ? r(z, { ref: a, autoFocus: true, value: t || "", placeholder: "Disabled", onFocus: s, classes: { root: d.input } }) : null;
}, Be = P()({ input: { position: "absolute", left: "0", width: "0px", opacity: "0" } });
function V(n) {
  return p.forwardRef((e, t) => r("span", { ref: t, children: r(n, { ...e }) }));
}
const Ke = V(ke), X = (n) => {
  const { dark: e, tipPosition: t, tipMargin: o, ...c } = n, { classes: s } = Ee({ tipMargin: o });
  return r(ae, { ...c, classes: { popper: K(s.popper, { "termius-dark-theme": e }), tooltip: s.tooltip }, title: N(_.Fragment, { children: [r(Ke, { className: K(s.tipIcon, s[t]) }), n.title] }) });
};
X.defaultProps = { tipPosition: "center", tipMargin: 3 };
const Ee = P()((n, e) => ({ tooltip: { position: "relative", background: "var(--card-c)", color: "var(--main-color)", padding: "12px 13px", borderRadius: "6px", fontSize: "12px", fontWeight: 450, lineHeight: "15px", boxShadow: `0px 5px 5px -3px rgba(0, 0, 0, 0.2),
      0px 8px 10px 1px rgba(0, 0, 0, 0.14),
      0px 3px 14px 2px rgba(0, 0, 0, 0.12)` }, tipIcon: { fill: "var(--card-c)", position: "absolute", height: "11px", width: "20px", top: "-9px" }, left: { left: `${e.tipMargin}px` }, right: { right: `${e.tipMargin}px` }, center: { left: "calc(50% - 10px)" }, popper: { opacity: "1", marginLeft: "9px", marginTop: "-5px", background: "none" } })), Ae = V(re), Oe = (n) => {
  const { conflictingWith: e } = n, { classes: t } = je();
  return e ? r(X, { open: true, tipPosition: "left", tipMargin: 2, placement: "bottom-start", title: `This will disable "${e}" shortcut`, children: r(Ae, { className: t.warningIcon }) }) : null;
}, je = P()({ warningIcon: { height: "100%", width: "14px", marginLeft: "15px" } });
function $e(n) {
  const { className: e, keys: t, keyClassName: o } = n, { classes: c, cx: s } = Le();
  return r("div", { className: s(c.shortcutPresenter, e), children: t.map((a) => r(ve, { className: K(o), children: a }, a)) });
}
const Le = P()({ shortcutPresenter: { display: "flex", gap: "10px" } }), Fe = (n) => {
  const { className: e, keys: t, endAdornment: o, isEditing: c } = n, { classes: s } = Re();
  return !t.length && !c ? r(E, { variant: "body2", color: "secondary", children: "Disabled" }) : N("div", { className: K(e, s.keyCombination), children: [c ? r(ie, { value: t.join("+"), inputProps: { className: s.input } }) : r($e, { keys: t, keyClassName: s.key }), o && r(E, { variant: "body2", color: "secondary", children: N("div", { className: s.additionalTextContainer, children: [r("div", { className: s.additionalText, children: "+" }), r("div", { className: s.additionalText, children: o })] }) })] });
}, Re = P()({ keyCombination: { display: "flex" }, key: { background: "var(--white)" }, input: { width: "100px", textAlign: "center" }, additionalTextContainer: { display: "flex", gap: 10 }, additionalText: { justifyContent: "center", display: "flex", alignItems: "center", minWidth: "22px", height: "22px" } }), J = [16, 17, 18, 91, 93], O = (n) => {
  const { onClick: e, isEditing: t, acceptOnlyModKey: o, getConflictingActionName: c, actionDescription: s, onChange: a, actionType: d, onFocus: h, onSubmitBinding: m, onCancelBinding: b, oldShortcut: v, endAdornment: x, disabled: y } = n, { classes: C } = qe(), [k, f] = p.useState([]), I = p.useCallback((l) => {
    l.preventDefault(), l.stopPropagation();
    const g = L(l);
    if (f([...k, l.keyCode]), !J.includes(l.keyCode) || o) {
      const T = g.length === 0, M = l.currentTarget;
      if (T) switch (l.keyCode) {
        case 13:
          m(), M == null || M.blur();
          return;
        case 8:
          a(null);
          return;
        case 27:
          a(v), M == null || M.blur(), b();
          return;
        default:
          return;
      }
      o || g.push(le(l));
    }
    a(g.join("+"));
  }, [o, v, a, k, f, b, m]), D = p.useCallback(ue(() => {
    if (o) return;
    k.filter((g) => !J.includes(g)).length || a(v);
  }, 150), [k, a, v, o]), i = he(n.keys, de) ?? [], S = t && n.keys && !o ? c(d) : "", u = p.useCallback(() => {
    t && m();
  }, [t, m]);
  return r(me, { onClickAway: u, children: N(pe, { className: K({ [C.listItem]: true, [C.active]: !!t }), onMouseDown: (l) => t && l.preventDefault(), onClick: (l) => !y && e(l), disableRipple: true, children: [N("div", { className: C.leftContainer, children: [r(Fe, { keys: i, isEditing: t, endAdornment: x }), r(Oe, { conflictingWith: S }), r(Te, { isEditing: t, keys: n.keys, keydown: I, keyup: D, onFocus: h })] }), r(E, { variant: "body2", color: "secondary", children: s })] }) });
};
O.defaultProps = { onChange: () => {
}, isEditing: false, actionType: "", onClick: () => {
}, acceptOnlyModKey: false, disabled: false, getConflictingActionName: () => {
}, oldShortcut: null, onFocus: () => {
}, onSubmitBinding: () => {
}, onCancelBinding: () => {
} };
const qe = P()({ listItem: { "--active-color": "var(--light-grey-4)", ".termius-dark-theme &": { "--active-color": "var(--dark-grey-3)" }, width: "100%", padding: "0 20px", height: "40px", display: "flex", alignItems: "center", justifyContent: "flex-start", "&:hover, &:focus": { background: "var(--active-color)" } }, active: { background: "var(--active-color)" }, leftContainer: { width: "210px", display: "flex" } }), Je = (n) => {
  var D, i, S;
  const { shortcutsManager: e, onSubmitShortcut: t, onStartEditingShortcut: o } = n, c = ge(), [s, a] = p.useState(), d = ((D = e.currentScheme) == null ? void 0 : D.value.mod) ?? null, h = ((i = e.currentScheme) == null ? void 0 : i.value.paneMod) ?? null, m = Object.fromEntries(((S = e.currentScheme) == null ? void 0 : S.value.shortcuts.map((u) => [u.type, u])) ?? []), b = p.useCallback(() => {
    if (s == null) return;
    const { actionType: u, oldShortcut: l, newShortcut: g, funnelID: T } = s;
    if (l === g) return;
    const { title: M, canBeDisabled: U, setShortcutImpl: Y } = He(u, m, e);
    g == null && !U || (Y(g), t(M, l, g, T));
  }, [s, t, m, e]), v = p.useCallback(() => {
    R.bind(void 0);
  }, []), x = p.useCallback(() => {
    var u;
    R.bind((u = e.currentScheme) == null ? void 0 : u.value);
  }, [e]);
  if (!Object.keys(c).length || !Object.keys(m).length) return null;
  const y = (u) => {
    if (s == null) return null;
    const { newShortcut: l } = s, g = Object.keys(c).find((T) => m[T].shortcut === l && T !== u);
    return g ? c[g].actionName : null;
  }, C = (u, l) => {
    if (u !== (s == null ? void 0 : s.actionType)) {
      const g = Se.v4();
      o(l, g), a({ actionType: u, oldShortcut: l, newShortcut: l, funnelID: g });
    }
  }, k = (s == null ? void 0 : s.actionType) === "mod", f = (s == null ? void 0 : s.actionType) === "paneMod", I = (u) => {
    a((l) => ($(l != null), B(l, (g) => {
      g.newShortcut = u;
    })));
  };
  return N(_.Fragment, { children: [r(O, { onClick: () => {
    C("mod", d);
  }, isEditing: k, actionDescription: "Switch to specific Tab [1…9]", endAdornment: "[1…9]", acceptOnlyModKey: true, keys: k ? s.newShortcut : d, onCancelBinding: () => {
    a(void 0);
  }, onSubmitBinding: () => {
    b(), a(void 0);
  }, onChange: I }), r(O, { onClick: () => {
    C("paneMod", h);
  }, isEditing: f, actionDescription: "Move focus between Split View panes", endAdornment: "arrows", keys: f ? s.newShortcut : h, onCancelBinding: () => {
    a(void 0);
  }, onSubmitBinding: () => {
    b(), a(void 0);
  }, onChange: I }), Object.keys(c).map((u) => {
    const l = u === (s == null ? void 0 : s.actionType);
    return r(O, { onClick: () => {
      C(u, m[u].shortcut);
    }, isEditing: l, oldShortcut: m[u].shortcut, actionDescription: c[u].actionName, keys: l ? s.newShortcut : m[u].shortcut, actionType: u, getConflictingActionName: y, onFocus: v, onSubmitBinding: () => {
      b(), x(), a(void 0);
    }, onCancelBinding: () => {
      x(), a(void 0);
    }, onChange: I }, u);
  })] });
};
function He(n, e, t) {
  return (/* @__PURE__ */ new Map([["mod", { title: "Switch to specific Tab mod", canBeDisabled: false, setShortcutImpl: (c) => {
    $(c != null, "Tab mod cannot be disabled"), t.changeTabsModifier(c);
  } }], ["paneMod", { title: "Move focus between Split View panes", canBeDisabled: false, setShortcutImpl: (c) => {
    $(c != null, "Panes mod cannot be disabled"), t.changePanesModifier(c);
  } }]])).get(n) ?? { title: e[n].actionName, canBeDisabled: true, setShortcutImpl: (c) => {
    t.changeShortcutByType({ shortcut: c, type: n });
  } };
}
function H(n) {
  return { scheme: n == null ? void 0 : n.name, schemeNew: (n == null ? void 0 : n.name) ?? "Disabled" };
}
const We = () => {
  const { current: n } = fe(() => new Ie()), e = n.shortcutSchemes, [t, o] = p.useState(() => H(n.currentScheme)), [c, s] = p.useState(""), a = ye(), d = p.useCallback(() => {
    const { currentScheme: i } = n;
    o(H(i)), a(be(i == null ? void 0 : i.value));
  }, [a, n]), h = () => {
    o({ ...t, scheme: c, schemeNew: c });
  }, m = (i) => {
    i.target.select(), s(i.target.value);
  }, b = (i) => i && e[i].editable, v = (i) => {
    n.deleteScheme(i), d();
  }, x = () => {
    t.schemeNew === "" || n.defaultSchemesNames.includes(t.schemeNew) ? h() : t.scheme != null && t.scheme !== t.schemeNew && n.renameScheme(t.scheme, t.schemeNew), d();
  }, y = (i) => {
    const S = i.currentTarget;
    i.keyCode === 13 && L(i).length === 0 && (S == null || S.blur()), i.keyCode === 27 && L(i).length === 0 && (h(), S == null || S.blur());
  };
  p.useEffect(() => {
    t.scheme && d();
  }, []);
  const C = p.useCallback((i) => {
    n.selectScheme(i === "Disabled" ? void 0 : i), d();
  }, [d, n]), k = Object.entries(e).map(([i, S]) => ({ name: i, id: i, ...S })), { classes: f } = ze(), I = p.useCallback((i, S, u, l) => {
    d(), a(q("shortcutChanged", { shortcutActionTitle: i, previousKeyCombination: S ?? "Disabled", currentKeyCombination: u ?? "Disabled", shortcutModificationFunnelID: l }));
  }, [a, d]), D = p.useCallback((i, S) => {
    a(q("shortcutEditingStarted", { currentKeyCombination: i ?? "Disabled", shortcutModificationFunnelID: S }));
  }, [a]);
  return r(Ce, { children: N("div", { className: f.root, children: [N("div", { className: f.pageHeader, children: [N("div", { className: f.schemeMenu, children: [r(z, { disabled: !b(t.scheme), onChange: (i) => {
    o({ ...t, schemeNew: i.target.value });
  }, placeholder: c, onFocus: m, onKeyUp: y, value: t.schemeNew, onBlur: x, classes: { root: f.schemeInput } }), r(Pe, { onDeleteScheme: v, schemes: k, onChange: C })] }), t.scheme && N("div", { className: f.shortcutsHeader, children: [r("div", { className: f.leftContainer, children: r(E, { variant: "subtitle1", color: "main", children: "Shortcut" }) }), r("div", { children: r(E, { variant: "subtitle1", color: "main", children: "Action" }) })] })] }), t.scheme && r(Je, { shortcutsManager: n, onStartEditingShortcut: D, onSubmitShortcut: I })] }) });
}, Ye = We, ze = P()({ root: { padding: "30px 0" }, schemeInput: { fontWeight: 500, fontSize: "16px", marginRight: "10px", "--text-color": "var(--main)", "&&:before, &&:hover:before": { borderBottomColor: "var(--text-color)" }, "&& input": { WebkitTextFillColor: "var(--text-color)" } }, pageHeader: { padding: "0 20px" }, schemeMenu: { display: "flex", alignItems: "center" }, shortcutsHeader: { marginTop: "15px", height: "36px", display: "flex", alignItems: "center" }, leftContainer: { width: "210px" } });
export {
  Ye as default
};
