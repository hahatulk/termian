/*
 * Recovered Termius UI module: HostsScreenSidebar-ff6de4d7.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { AE as Xe, AF as Ze, AG as Qe, bq as X, cN as le, bj as H, ez as en, gG as nn, bZ as tn, lJ as U, r as t, cl as J, AH as on, ef as pe, eg as Z, AI as sn, AJ as _e, AK as ye, aC as an, f1 as ke, f2 as Te, gm as ln, aH as Ne, d5 as ce, bG as ue, eh as fe, bi as Ae, g2 as ne, bt as c, e8 as se, AL as rn, AM as cn, br as B, bO as ge, AN as un, pI as dn, AO as ae, eX as we, AP as hn, bA as oe, AQ as gn, fa as pn, AR as fn, AS as mn, AT as Cn, ew as Fe, AU as Sn, AV as bn, AW as vn, AX as In, mj as _n, e0 as yn, nB as kn, hy as Tn, ml as Nn, AY as An, AZ as wn, A_ as Fn, A$ as Vn, f9 as Se, B0 as On, ge as xn, bu as de, kD as En, bU as Pn, fY as he, fv as be, B1 as Hn, B2 as Bn, kI as Mn, B3 as Gn, mk as Rn, kF as Dn, B4 as Ln, B5 as Wn, B6 as Un, er as Ve, B7 as jn, cv as zn, fT as qn, B8 as $n, k6 as Kn, ed as Oe, eu as Yn, ev as Jn, ck as Xn, ex as Zn, ee as Qn, ei as et, ej as nt, ea as tt, ec as ot, jR as st, eb as at, eG as lt, jS as rt, jW as it, jT as ct, ek as ut, el as dt, dk as ht, dn as gt, em as pt, dl as ft, en as mt, eo as Ct, ep as St, eq as bt, et as vt, jf as It, ba as _t, dP as yt, eF as kt, dS as Tt, dT as ve } from "./reconnectSaga-3b11f62e.js";
import { u as Nt, r as At, a as wt, M as Ie, b as Ft, S as Vt, c as Ot, s as xt, N as Et } from "./ui-process-62a480c2.js";
import { M as Pt } from "./openExternal.react-f073251d.js";
import "./watchAction-2425dc2f.js";
import "./FontAutocomplete-0a9583b4.js";
import "./useMediaQuery-22669263.js";
import "./index-bc78ea9d.js";
import "./Divider-8ca798ff.js";
import "./UserListItem-e815dc7d.js";
import "./PromoBanner-aac31425.js";
import "./triggerChoosePlanDialog-b93628a0.js";
import "./AlertDialog-b50b9ed7.js";
import "./InputDialog-b3f32c9f.js";
import "./KeyboardKey-4328b011.js";
import "./retry-circular.react-77791386.js";
import "./FullscreenDialogFooter-3b4c7076.js";
import "./FullScreenDialogLayout-e1c06c79.js";
import "./LinkButton-03e176fc.js";
import "./PageWrapper-a7d0cf6b.js";
import "./index-a4ab4f8f.js";
import "./PortForwardingRulePresenter-4942c31a.js";
import "./SettingsSwitch-d35e9651.js";
import "./External Link.react-a5589d4c.js";
import "./ErrorOutline-9dd3d7a0.js";
import "./useHandledClick-9dba862b.js";
const Ht = [{ name: "Auto", value: Xe }, { name: "IPv4", value: Ze }, { name: "IPv6", value: Qe }], Bt = (r) => {
  var me;
  const { config: e, onConfigChange: n, isVisible: i, credentialsMode: a } = r, b = X(), C = le(), v = H(en), m = H(nn), h = H(tn) && !m, [g, p] = U(i), [y, w] = t.useState(J.v4), [d, S] = t.useState(J.v4), [I, N] = t.useState(J.v4), [o, l] = U(i), [T, _] = U(i), [F, k] = U(i), [u, f] = U(i), [x, E] = U(i), [R, M] = t.useState(), $ = t.useCallback(() => {
    M(void 0);
  }, []);
  on($, i);
  const G = t.useCallback(async (s) => {
    const V = s ? new pe(r.config || void 0) : null;
    V && !V.identity && (V.identity = new Z({ is_visible: false })), V == null || V.stripEmptyIdentity(), n({ fieldName: "ssh_config", fieldValue: V });
  }, [r.config, n]), j = t.useCallback((s) => {
    n({ fieldName: "ssh_config.use_mosh", fieldValue: s }), e != null && e.mosh_server_command || n({ fieldName: "ssh_config.mosh_server_command", fieldValue: sn });
  }, [e == null ? void 0 : e.mosh_server_command, n]), K = t.useCallback((s) => {
    const V = (typeof s == "string" ? s : s.target.value) || null;
    n({ fieldName: "ssh_config.mosh_server_command", fieldValue: V });
  }, [n]), Y = t.useCallback((s) => {
    n({ fieldName: "ssh_config.port", fieldValue: s });
  }, [n]), z = t.useCallback((s) => {
    s != null && s.is_visible && b(_e({ bannerOpenState: "closed", configType: "ssh", configLocalId: e == null ? void 0 : e.local_id }));
    let V = null;
    s && !s.isEmpty() && (V = s != null && s.local_id ? s : new Z({ ...s, ...ye(a, e == null ? void 0 : e.encrypted_with), is_visible: !!(s != null && s.is_visible) })), n({ fieldName: "ssh_config.identity", fieldValue: V });
  }, [e == null ? void 0 : e.encrypted_with, e == null ? void 0 : e.local_id, a, b, n]), A = t.useCallback(async (s) => {
    n({ fieldName: "ssh_config.startup_snippet", fieldValue: s }), v || await te({ sagas: C, from: "StartupSnippet", featureName: "Automation", funnelID: y, funnelIsStarted: true });
  }, [v, n, C, y]), D = t.useCallback(() => {
    n({ fieldName: "ssh_config.startup_snippet", fieldValue: null });
  }, [n]), P = t.useCallback((s) => {
    n({ fieldName: "ssh_config.charset", fieldValue: s.label });
  }, [n]), L = t.useCallback(() => {
    n({ fieldName: "ssh_config.charset", fieldValue: null });
  }, [n]), q = t.useCallback(async (s) => {
    n({ fieldName: "ssh_config.agent_forwarding", fieldValue: s }), !v && s && await te({ sagas: C, from: "AgentForwarding", featureName: "Forwarding" });
  }, [v, C, n]), Q = t.useCallback(() => {
    n({ fieldName: "ssh_config.agent_forwarding", fieldValue: null });
  }, [n]), ee = t.useCallback(async (s) => {
    n({ fieldName: "ssh_config.host_chain", fieldValue: s }), v || await te({ sagas: C, from: "HostChaining", featureName: "Tunneling", funnelID: d, funnelIsStarted: true });
  }, [d, v, n, C]), re = t.useCallback(() => {
    n({ fieldName: "ssh_config.host_chain", fieldValue: null });
  }, [n]), O = t.useCallback(async (s) => {
    n({ fieldName: "ssh_config.proxy", fieldValue: s }), v || await te({ sagas: C, from: "Proxy", featureName: "Tunneling", funnelID: I, funnelIsStarted: true });
  }, [v, n, I, C]), W = t.useCallback(() => {
    n({ fieldName: "ssh_config.proxy", fieldValue: null });
  }, [n]), xe = () => {
    E(false), M(null);
  }, Ee = t.useCallback(async (s) => {
    r.config && (n({ fieldName: "ssh_config.env_variables", fieldValue: s }), E(false));
  }, [r.config, E, n]), Pe = t.useCallback((s) => {
    var Ce;
    let V = (Ce = r.config) != null && Ce.env_variables ? [...r.config.env_variables] : null;
    V == null || V.splice(s, 1), V = (V == null ? void 0 : V.length) === 0 ? null : V, n({ fieldName: "ssh_config.env_variables", fieldValue: V });
  }, [(me = r.config) == null ? void 0 : me.env_variables, n]), ie = t.useCallback((s) => {
    n({ fieldName: "ssh_config.color_scheme", fieldValue: s });
  }, [n]), He = t.useCallback(() => {
    n({ fieldName: "ssh_config.color_scheme", fieldValue: null });
  }, [n]), Be = t.useCallback((s) => {
    E(true), M(s);
  }, [E]), Me = () => {
    const s = J.v4();
    b(ce({ from: "StartupSnippet", upgradeToProFunnelID: s, trial: ue(h) })), p(true), w(s);
  }, Ge = () => {
    p(false);
  }, Re = () => {
    f(true);
  }, De = () => {
    f(false);
  }, Le = () => {
    const s = J.v4();
    b(ce({ from: "HostChaining", upgradeToProFunnelID: s, trial: ue(h) })), _(true), S(s);
  }, We = () => {
    _(false);
  }, Ue = t.useCallback((s) => {
    ee(s), _(false);
  }, [ee, _]), je = () => {
    const s = J.v4();
    b(ce({ from: "Proxy", upgradeToProFunnelID: s, trial: ue(h) })), l(true), N(s);
  }, ze = () => {
    l(false);
  }, qe = t.useCallback((s) => {
    O(s), l(false);
  }, [O, l]), $e = () => {
    E(true);
  }, Ke = () => {
    k(true);
  }, Ye = () => {
    k(false);
  }, Je = t.useCallback((s) => {
    ie(s), k(false);
  }, [ie, k]);
  return { snippetsOpen: g, proxyOpen: o, hostChainOpen: T, colorSchemeOpen: F, charsetOpen: u, environmentVariablesOpen: x, variableIndex: R, onPortChange: Y, onIdentityChange: z, onEnabledChange: G, onUseMoshChange: j, onMoshServerCommandChange: K, onStartupSnippetChange: A, onRemoveStartupSnippet: D, onCharsetChange: P, onRemoveCharset: L, onAgentForwardingChange: q, onRemoveAgentForwarding: Q, onHostChainChange: ee, onRemoveHostChain: re, onProxyChange: O, onRemoveProxy: W, onEnvironmentVariablesSubmit: Ee, onEnvironmentVariablesRemove: Pe, onColorSchemeChange: ie, onRemoveColorScheme: He, handleEnvironmentVariablesOpen: $e, handleEnvironmentVariablesOpenWithVariable: Be, onColorSchemeSubmit: Je, handleColorSchemeOpen: Ke, handleColorSchemeClose: Ye, handleEnvironmentVariablesClose: xe, onProxySubmit: qe, handleProxyClose: ze, handleProxyOpen: je, onHostChainSubmit: Ue, handleHostChainOpen: Le, handleHostChainClose: We, handleCharsetClose: De, handleCharsetOpen: Re, handleSnippetsClose: Ge, handleSnippetsOpen: Me };
}, te = async (r) => {
  const { sagas: e, from: n, featureName: i, funnelID: a, funnelIsStarted: b } = r;
  await an(1e3), ke(e, { dialog: "ProFeatures", target: Te(), props: { openWithNumber: ln(i), from: n, funnelIsStarted: b, funnelID: a } }).catch(Ne);
}, Mt = (r) => {
  const { config: e, onConfigChange: n, isVisible: i, credentialsMode: a } = r, b = X(), [C, v] = U(i), [m, h] = U(i), g = t.useCallback(async (u) => {
    const f = u ? new fe(r.config || void 0) : null;
    f && !f.identity && (f.identity = new Z({ is_visible: false })), f == null || f.stripEmptyIdentity(), n({ fieldName: "telnet_config", fieldValue: f });
  }, [r.config, n]), p = t.useCallback((u) => {
    n({ fieldName: "telnet_config.port", fieldValue: u });
  }, [n]), y = t.useCallback((u) => {
    u != null && u.is_visible && b(_e({ bannerOpenState: "closed", configType: "telnet", configLocalId: e == null ? void 0 : e.local_id }));
    let f = null;
    u && !u.isEmpty() && (f = u != null && u.local_id ? u : new Z({ ...u, ...ye(a, e == null ? void 0 : e.encrypted_with), is_visible: !!(u != null && u.is_visible) })), n({ fieldName: "telnet_config.identity", fieldValue: f });
  }, [e == null ? void 0 : e.local_id, e == null ? void 0 : e.encrypted_with, a, b, n]), w = t.useCallback((u) => {
    n({ fieldName: "telnet_config.startup_snippet", fieldValue: u });
  }, [n]), d = t.useCallback(() => {
    n({ fieldName: "telnet_config.startup_snippet", fieldValue: null });
  }, [n]), S = t.useCallback((u) => {
    n({ fieldName: "telnet_config.charset", fieldValue: u.label });
  }, [n]), I = t.useCallback(() => {
    n({ fieldName: "telnet_config.charset", fieldValue: null });
  }, [n]), N = t.useCallback((u) => {
    n({ fieldName: "telnet_config.color_scheme", fieldValue: u });
  }, [n]), o = t.useCallback(() => {
    n({ fieldName: "telnet_config.color_scheme", fieldValue: null });
  }, [n]), l = () => {
    h(true);
  }, T = () => {
    h(false);
  }, _ = () => {
    v(true);
  }, F = () => {
    v(false);
  }, k = t.useCallback((u) => {
    N(u), v(false);
  }, [N, v]);
  return { colorSchemeOpen: C, charsetOpen: m, onPortChange: p, onIdentityChange: y, onEnabledChange: g, onStartupSnippetChange: w, onRemoveStartupSnippet: d, onCharsetChange: S, onRemoveCharset: I, onColorSchemeChange: N, onRemoveColorScheme: o, onColorSchemeSubmit: k, handleColorSchemeOpen: _, handleColorSchemeClose: F, handleCharsetClose: T, handleCharsetOpen: l };
};
function Gt(r) {
  const { host: e, shouldDisableFormFields: n, isVisible: i, canEditIdentity: a, onChangeCredentialsMode: b, onSshConfigChange: C, onTelnetConfigChange: v, isOpenedFromEmptyScreen: m, onCreateGroup: h, shouldShowAllSshConfigs: g = false, shouldShowAllTelnetConfigs: p = false, onTelnetAndSshConfigChange: y } = r, { classes: w } = Rt(), d = e.getMergedSshConfig(), S = e.getMergedTelnetConfig(), [I, N] = t.useState(g), [o, l] = t.useState(p), T = Bt({ config: e.ssh_config, onConfigChange: C, isVisible: i, credentialsMode: e.credentials_mode }), _ = Mt({ config: e.telnet_config, onConfigChange: v, isVisible: i, credentialsMode: e.credentials_mode }), [F, k] = t.useState(/* @__PURE__ */ new Map([["ssh", !!d.current], ["telnet", !!S.current]])), u = () => {
    l(false), _.onEnabledChange(false), k(ne((A) => {
      A.delete("telnet");
    }));
  }, f = () => {
    N(false), T.onEnabledChange(false), k(ne((A) => {
      A.delete("ssh");
    }));
  }, x = () => {
    l(true), _.onEnabledChange(true), k(ne((A) => {
      A.set("telnet", true);
    }));
  }, E = () => {
    N(true), T.onEnabledChange(true), k(ne((A) => {
      A.set("ssh", true);
    }));
  }, R = (A, D) => {
    const { identity: P, saveToSecondConfig: L } = A;
    if (L) {
      y({ ssh: new pe({ ...d.current, identity: P }), telnet: new fe({ ...S.current, identity: P }) });
      return;
    }
    D === "ssh" ? T.onIdentityChange(P) : _.onIdentityChange(P);
  }, M = (A) => {
    R(A, "ssh");
  }, $ = (A) => {
    R(A, "telnet");
  };
  t.useEffect(() => {
    N(g), l(p);
  }, [g, p]), t.useEffect(() => {
    k(ne((A) => {
      A.set("ssh", !!d.current), A.set("telnet", !!S.current);
    }));
  }, [d, S]);
  const G = d.current ? c(se, { className: w.formCardPadding, "aria-label": "Ssh config form", role: "form", children: c(rn, { sshConfig: d, disabled: n, canEditIdentity: a, root: e, onChangeCredentialsMode: b, onRemoveConfig: f, sshConfigForm: T, originalEntity: e, isNewProtocol: I || !!m, originalConfig: d, onCreateGroup: h, onIdentityCreated: M }) }) : null, j = S.current ? c(se, { className: w.formCardPadding, "aria-label": "Telnet config form", role: "form", children: c(cn, { telnetConfig: S, disabled: n, canEditIdentity: a, root: e, onChangeCredentialsMode: b, onRemoveConfig: u, telnetConfigForm: _, originalEntity: e, isNewProtocol: o, originalConfig: S, onCreateGroup: h, onIdentityCreated: $ }) }) : null, K = !(d.current && S.current), Y = !e.local_id && K, z = (K || Y) && !n;
  return B(ge.Fragment, { children: [[...F].map(([A, D]) => {
    if (D) {
      const P = A === "ssh" && d.current, L = A === "telnet" && S.current;
      return B(ge.Fragment, { children: [P && G, L && j] }, A);
    }
    return null;
  }), z && c(un, { sshConfig: d, telnetConfig: S, onSshEnabledChange: E, onTelnetEnabledChange: x })] });
}
const Rt = Ae()({ formCardPadding: { padding: "15px 20px" } }), Dt = (r) => {
  const { updateFieldValue: e, editableEntity: n, shouldDisableFormFields: i, handleSubmit: a } = r, b = X(), C = H(dn), v = t.useCallback((o) => {
    const l = o.map((T) => ae(T) ? T : T.label);
    e({ fieldName: "tags", fieldValue: l }), a({ useAutosave: true, triggerSource: "explicit" });
  }, [e, a]), m = t.useCallback((o) => {
    i || (e({ fieldName: "group", fieldValue: o ? new we(o) : null }), a({ useAutosave: true, triggerSource: "explicit" }));
  }, [i, e, a]), h = t.useCallback(() => {
    i || (e({ fieldName: "group", fieldValue: null }), a({ useAutosave: true, triggerSource: "explicit" }));
  }, [i, e, a]), g = t.useCallback((o, l) => {
    if (i) return;
    const { ssh_config: T, telnet_config: _ } = l;
    e({ fieldName: "group", fieldValue: o }), T && e({ fieldName: "ssh_config", fieldValue: T }), _ && e({ fieldName: "telnet_config", fieldValue: _ }), a({ useAutosave: true, triggerSource: "explicit" });
  }, [i, e, a]), p = t.useCallback((o) => {
    e({ fieldName: "backspace", fieldValue: o ? "legacy" : "default" }), a({ useAutosave: true, triggerSource: "explicit" });
  }, [e, a]), y = t.useCallback((o) => {
    const l = hn(o);
    l && b(oe("terminalThemeChanged", { terminalTheme: l, terminalThemeSource: "Host Edit", globalTerminalTheme: C, isWorkspace: "no", workspaceTemplateID: void 0 }));
  }, [C, b]), w = t.useCallback((o) => {
    e(o), o.fieldName === "ssh_config.color_scheme" && y(o.fieldValue), a({ useAutosave: true, triggerSource: "explicit" });
  }, [e, y, a]), d = t.useCallback((o) => {
    e(o), o.fieldName === "telnet_config.color_scheme" && y(o.fieldValue), a({ useAutosave: true, triggerSource: "explicit" });
  }, [e, y, a]), S = t.useCallback(({ telnet: o, ssh: l }) => {
    e({ fieldName: "telnet_config", fieldValue: o }), e({ fieldName: "ssh_config", fieldValue: l }), y(o == null ? void 0 : o.color_scheme), a({ useAutosave: true, triggerSource: "explicit" });
  }, [e, y, a]), I = t.useCallback((o) => {
    const l = gn(n, o);
    e({ fieldName: "credentials_mode", fieldValue: l.credentials_mode }), e({ fieldName: "ssh_config", fieldValue: l.ssh_config }), e({ fieldName: "telnet_config", fieldValue: l.telnet_config }), a({ useAutosave: true, triggerSource: "explicit" });
  }, [e, n, a]);
  return { canEditIdentity: pn(n, i), onTagsChange: v, onGroupChange: m, onRemoveGroup: h, onChangeGroupAndConfig: g, onBackspaceChange: p, onSshConfigChange: w, onTelnetConfigChange: d, onTelnetAndSshConfigChange: S, onChangeCredentialsMode: I };
}, Lt = ({ editableEntity: r, onGroupChange: e, onChangeGroupAndConfig: n, shouldDisableFormFields: i }) => {
  const a = le(), [b, C] = t.useReducer(fn, mn), [v, m] = U(false), h = t.useCallback(() => {
    i || m(true);
  }, [m, i]), g = t.useCallback(() => {
    m(false);
  }, [m]), p = t.useCallback(({ config: I, hostsList: N = [], fieldToRemoveFromConfig: o }) => {
    C(Cn({ config: I, hostsList: N, fieldToRemoveFromConfig: o })), h();
  }, [h]), y = t.useCallback((I) => {
    (I == null || I instanceof we) && e(I);
  }, [e]), w = t.useCallback(async (I, N) => {
    var u;
    const o = !!r.local_id, l = N == null ? void 0 : N.filter((f) => !!f.local_id), T = l == null ? void 0 : l.find((f) => f.local_id === r.local_id);
    o && !T && (l == null || l.push(r));
    const { fieldToRemoveFromConfig: _ } = b;
    if (!_) {
      e(I);
      return;
    }
    const { configType: F, configField: k } = _;
    if (l) {
      const f = l.map((x) => {
        var E;
        return (E = x[F]) == null || E.resetField(k), x;
      });
      try {
        await Fe(a, Sn, { hosts: f, groups: [], targetGroup: I });
      } catch {
      }
    }
    o || ((u = r[F]) == null || u.resetField(k), n(I, { [F]: r[F] }));
  }, [r, n, e, a, b]), d = t.useCallback((I = "") => {
    C(bn({ groupName: I })), h();
  }, [h]), S = t.useCallback(() => {
    C(vn()), h();
  }, [h]);
  return { state: b, groupSliderOpened: v, onOpenGroupsSlider: h, onCloseGroupsSlider: g, onCreateGroupWithConfig: p, onAutocompleteGroupChange: y, groupChangeHandler: w, onGroupCreate: d, handleGroupOpen: S };
}, Wt = ({ editableEntity: r, onTagsChange: e, hosts: n }) => {
  const i = X(), a = H(In), b = H(_n), [C, v] = t.useState(""), h = yn([...a, ...b], (o) => ae(o) ? o : o.label).map((o) => ae(o) ? { type: "tag", value: o, label: o } : { type: "snippetsPackage", value: o, label: o.displayLabel }), g = kn(r.tags).map((o) => ({ type: "tag", value: o, label: o })), p = t.useRef([]), y = t.useCallback((o, l) => {
    if (o && Tn(o)) {
      const T = o.map((_) => {
        const { type: F, value: k } = _;
        return F === "snippetsPackage" ? new Nn(k) : k;
      });
      if (e(T), l.action === "create-option" && i(An()), l.action === "select-option") {
        const _ = n.filter((k) => {
          var u, f;
          return k.tags.includes(((u = l.option) == null ? void 0 : u.type) === "snippetsPackage" ? l.option.value.displayLabel : ((f = l.option) == null ? void 0 : f.value) || "");
        }).length, F = p.current.findIndex((k) => {
          var u;
          return k.label === ((u = l.option) == null ? void 0 : u.label);
        });
        i(wn({ position: F, numberOfSymbols: C.length, inputFieldType: "Tag", suggestedEntity: "Tag", numberOfLinkedElements: _, numberOfSuggestions: p.current.length }));
      }
    }
  }, [C.length, i, n, e]), w = t.useCallback(() => {
    p.current.length && i(Fn({ numberOfSymbols: g.reduce((o, l) => o + (l.type === "snippetsPackage" ? l.value.displayLabel.length : l.value.length), 0), numberOfSuggestions: p.current.length, inputFieldType: "Tag", suggestedEntity: "Tag" }));
  }, [i, g]), d = t.useCallback(() => {
    i(Vn({ inputFieldType: "Tag" }));
  }, [i]), S = t.useCallback((o) => {
    v(o);
  }, []), I = t.useCallback((o, l, T, _) => {
    const F = T.filter((k) => !l.find((u) => _.getOptionValue(k) === u.label) && _.getOptionValue(k).toLocaleLowerCase().includes(o.toLocaleLowerCase()));
    return !F.length && !o ? (p.current = [], false) : (p.current = F, !F.length);
  }, []), N = t.useCallback(() => (p.current = [], null), []);
  return { tagsAndPackagesOptions: h, hostTagsOptions: g, handleTagsChange: y, handleMenuOpen: w, handleFocus: d, handleInputChange: S, isValidNewOption: I, noOptionsMessage: N };
}, Ut = ({ shouldDisableFormFields: r, isVisible: e, shareSlot: n = null, onConnect: i = () => {
}, isOpenedFromEmptyScreen: a, isSubForm: b, connectLabel: C = "Connect", shouldShowAllSshConfigs: v, shouldShowAllTelnetConfigs: m, savingStatus: h, editableEntity: g, updateFieldValue: p, handleSubmit: y, control: w }) => {
  const { classes: d } = jt(), S = Dt({ editableEntity: g, updateFieldValue: p, shouldDisableFormFields: r, handleSubmit: y }), { canEditIdentity: I, onTagsChange: N, onGroupChange: o, onRemoveGroup: l, onChangeGroupAndConfig: T, onBackspaceChange: _, onChangeCredentialsMode: F } = S, k = Se("groups").filter((O) => !!O.displayLabel), u = H(On), f = Se("hosts"), { state: x, groupSliderOpened: E, onCloseGroupsSlider: R, onCreateGroupWithConfig: M, onAutocompleteGroupChange: $, groupChangeHandler: G, onGroupCreate: j, handleGroupOpen: K } = Lt({ editableEntity: g, onGroupChange: o, onChangeGroupAndConfig: T, shouldDisableFormFields: r }), { tagsAndPackagesOptions: Y, hostTagsOptions: z, handleTagsChange: A, handleMenuOpen: D, handleFocus: P, handleInputChange: L, isValidNewOption: q, noOptionsMessage: Q } = Wt({ editableEntity: g, onTagsChange: N, hosts: f }), ee = h === "saving" || h === "editing", re = !g.address || !g.local_id || ee;
  return B(ge.Fragment, { children: [B(xn, { className: d.formPaddings, children: [B(se, { className: d.formCardPadding, children: [c(de, { color: "main", className: d.titleSection, children: "Address" }), B("div", { className: d.entityIconBlock, children: [c(En, { icon: Pn.icon(g), className: d.entityIcon }), c(he, { name: "address", control: w, rules: { required: true }, render: ({ field: { ref: O, ...W } }) => c(be, { ...W, placeholder: "IP or Hostname", disabled: r, fullWidth: true, className: d.inputField, InputProps: { inputRef: O }, autoFocus: !g.local_id }) })] }), Hn.value() && c(he, { name: "ip_version", control: w, render: ({ field: { ref: O, ...W } }) => c(Bn, { ...W, title: "IP Version", items: Ht }) })] }), B(se, { className: d.formCardPadding, children: [c(de, { color: "main", className: d.titleSection, children: "General" }), c(he, { name: "label", control: w, render: ({ field: { ref: O, ...W } }) => c(be, { ...W, placeholder: "Label", disabled: r, fullWidth: true, className: d.inputField }) }), c(Mn, { entityOptions: k, startAdornment: c(Gn, { className: d.fieldIcon }), autocompleteFieldName: "group", editableEntity: g, autocompleteValue: g.group, onRemove: l, changeEntity: $, createNewEntity: j, handleGroupOpen: K, createEntityLabel: "Group", autocompletePlaceholder: "Parent Group", disabled: r }), c(Rn, { options: Y, value: z, onChange: A, getOptionValue: (O) => ae(O.value) ? O.value : O.value.label, getNewOptionData: (O) => ({ type: "tag", value: O, label: B(de, { color: "secondary", children: ["Create Tag", " ", c("span", { className: d.createOption, children: O })] }) }), placeholder: "Tags", noOptionsMessage: Q, isDisabled: r, isMulti: true, startAdornment: c(Dn, { position: "start", children: c(Ln, { className: d.fieldIcon }) }), onMenuOpen: D, openMenuOnFocus: true, tabSelectsValue: false, onFocus: P, isValidNewOption: q, onInputChange: L, filterOption: (O, W) => O.value.toLowerCase().includes(W.toLowerCase()), classNames: { rootClassName: d.tagsSelector, inputClassName: d.tagsInputSelector } }), c(Wn, { icon: c(Un, {}), title: "Backspace", isActive: g.backspace !== "default", menuItemHandler: _, enabledTitle: "Ctrl+H", disabledTitle: "Default", enableTitle: "Ctrl+H", disableTitle: "Default", disabled: r })] }), n, c(Gt, { ...S, host: g, shouldDisableFormFields: r, isVisible: e, canEditIdentity: I, onChangeCredentialsMode: F, isOpenedFromEmptyScreen: a, onCreateGroup: M, shouldShowAllSshConfigs: v, shouldShowAllTelnetConfigs: m }), c(Ve, { value: g, children: c(jn, { open: E, handleClose: R, onChange: G, groupName: x.groupName, parentGroup: g.group, isOpenRoot: x.rootGroupIsOpen, groupConfig: x.groupConfig, showBanner: u && x.groupFormOpensWithConfig, hostsList: x.hostsListToBeAttachedToGroup, currentHost: g, hideConfig: true }) })] }), !b && c("div", { className: d.connectButton, children: c(zn, { fullWidth: true, disabled: re, color: "accent", onClick: i, children: C }) })] });
}, jt = Ae()({ titleSection: { fontWeight: 500, display: "flex", alignItems: "center", justifyContent: "space-between" }, entityIcon: { width: "36px", height: "36px", borderRadius: "8px", "& svg": { maxWidth: "20px", maxHeight: "20px" } }, connectButton: { background: "var(--light-grey-6)", padding: "20px 15px", position: "absolute", right: 0, bottom: 0, width: "calc(100% - 1px)", zIndex: 1, ".termius-dark-theme &": { background: "var(--dark-grey-2)" } }, formPaddings: { padding: "15px calc(15px - var(--scrollbar-width)) 85px 15px", gap: "15px" }, fieldIcon: { color: "var(--light-grey-3)", ".termius-dark-theme &": { color: "var(--dark-grey-6)" } }, inputField: { "&&&& input": { "&::placeholder": { color: "var(--light-grey-3)", ".termius-dark-theme &": { color: "var(--dark-grey-6)" } } } }, entityIconBlock: { display: "flex", alignItems: "center" }, createOption: { color: "var(--main)" }, formCardPadding: { padding: "15px 20px 20px" }, tagsSelector: { "& .MuiInputAdornment-positionStart": { marginRight: "3px" }, "& .MuiInputBase-root": { padding: "0px" } }, tagsInputSelector: { "&&&&:hover": { "--border-color": "var(--light-grey-4)", "--input-text-hover-color": "var(--light-grey-3)", ".termius-dark-theme &": { "--border-color": "var(--dark-grey-5)", "--input-text-hover-color": "var(--dark-grey-6)" } } } }), zt = () => {
  const r = X(), e = le(), { onChangeVault: n } = Nt(), i = (m) => m.clone(), a = (m) => !!m.address, b = t.useCallback((m) => {
    const { savedEntity: h } = m;
    r(qn({ host: h }));
  }, [r]), C = t.useCallback(async (m) => {
    const h = J.v4();
    ke(e, { dialog: "ShareAndInvite", target: Te(), options: { closeOnBackdropClick: false }, props: { entitiesToShare: { hosts: [m] }, entryPointForAnalytics: "Edit Host", teamTrialFunnelID: h } }).catch(Ne), r(oe("shareButtonPressed", { sharingButtonLocation: "Host Edit Screen", sharedEntity: "Host" })), r(oe("createTeamWizardViewOpened", { createTeamWizardStep: "Description", openedFrom: "Edit Host", teamTrialFunnelID: h, trial: "yes" })), r(oe("teamTrialFunnelStarted", { openedFrom: "Edit Host", teamTrialFunnelID: h, teamTrialPlan: "Team" }));
  }, [r, e]), v = t.useCallback((m) => (h) => {
    const { editableEntity: g, updateFieldValue: p, handleSubmit: y } = m;
    n(h);
    const w = $n(g, h);
    Kn(w).forEach(([S, I]) => {
      p({ fieldName: S, fieldValue: I });
    }), p({ fieldName: "group", fieldValue: void 0 }), y({ useAutosave: true, triggerSource: "explicit" });
  }, [n]);
  return { isValidForSaving: a, handleEntitySaved: b, onShareHost: C, prefill: i, createVaultChanger: v };
}, qt = (r) => {
  const { editableEntity: e, editableVaults: n, disabledActions: i, connectToHost: a, onClose: b, setShouldShowAllSshConfigs: C, setShouldShowAllTelnetConfigs: v, updateFieldValue: m, handleSubmit: h, isEntitySaved: g } = r, p = X(), y = le(), w = Oe(e, n), d = t.useCallback(() => {
    a == null || a(e);
  }, [a, e]), S = t.useCallback(() => {
    p(Yn.request([e])), p(Jn({ entity: "Host", onScreen: "Edit Host" }));
  }, [p, e]), I = t.useCallback(async () => {
    const T = [e];
    try {
      await At({ groups: [], hosts: T });
    } catch (_) {
      Xn.swallow(_);
      return;
    }
    await b(), await Fe(y, Zn, { hosts: T, groups: [] });
  }, [e, b, y]), N = t.useCallback(() => {
    C(true), m({ fieldName: "ssh_config", fieldValue: new pe({ identity: new Z({ is_visible: false }) }) }), h({ useAutosave: true, triggerSource: "explicit" });
  }, [C, m, h]), o = t.useCallback(() => {
    v(true), m({ fieldName: "telnet_config", fieldValue: new fe({ identity: new Z({ is_visible: false }) }) }), h({ useAutosave: true, triggerSource: "explicit" });
  }, [v, m, h]);
  return t.useMemo(() => [{ visible: !!a, name: "Connect", handler: d, icon: c(Qn, {}), disabled: i }, { visible: w && wt({ sshConfigIsExisted: !!e.ssh_config, telnetConfigIsExisted: !!e.telnet_config, entityIsCreated: g }), name: "Add SSH", handler: N, icon: c(Ie, {}) }, { visible: w && Ft({ telnetConfigIsExisted: !!e.telnet_config, sshConfigIsExisted: !!e.ssh_config, entityIsCreated: g }), name: "Add Telnet", handler: o, icon: c(Ie, {}) }, { visible: w, name: "Duplicate", handler: S, icon: c(Pt, {}), disabled: i }, { visible: et(e, n), name: "Remove", handler: I, icon: c(nt, {}), disabled: i }], [a, d, i, w, e, g, N, o, S, n, I]);
}, $t = ["address", "port", "username", "password"], Kt = (r) => {
  const { className: e = "", isOpened: n, onClose: i, connectToHost: a, isOpenedFromEmptyScreen: b, connectLabel: C } = r, v = X(), m = H(tt), h = H(ot), [g, p] = t.useState(false), [y, w] = t.useState(false), d = st(), S = at(), I = H(lt[d].formNavigation), N = rt({ currentSliderFormNavigation: I }), o = H((q) => it.host(q, { page: d, formId: N })), { isValidForSaving: l, handleEntitySaved: T, onShareHost: _, prefill: F, createVaultChanger: k } = zt(), u = ct({ defaultValues: It, isValidForSaving: l, handleEntitySaved: T, formType: "host", entity: o, entityClass: _t, formId: N, prefill: F }), { editableEntity: f, savingStatus: x, isValid: E, updateFieldValue: R, handleSubmit: M } = u, $ = k({ editableEntity: f, updateFieldValue: R, handleSubmit: M }), G = !!f.local_id, j = !G || !!x && x !== "saved", Y = h ? c(Ot, { type: "host", onClick: () => {
    _(f);
  }, disabled: !E || !G }) : null, z = () => {
    a == null || a(f);
  }, A = (q) => {
    if (j || q.key.toLowerCase() !== "enter") return;
    const Q = q.target.name;
    $t.includes(Q) && z();
  }, D = Oe(f, m), P = qt({ editableEntity: f, editableVaults: m, disabledActions: j, connectToHost: a, onClose: i, setShouldShowAllSshConfigs: p, setShouldShowAllTelnetConfigs: w, updateFieldValue: R, handleSubmit: M, isEntitySaved: G }), L = () => {
    S.openMenu(), v(yt({ onScreen: "Edit Host", openedVia: "Three Dots Button", wizardID: void 0 }));
  };
  return c(ut, { current: f, children: B(dt, { onKeyUp: A, onEsc: i, className: e, handleEnterKey: true, hasFormTag: true, children: [c(ht, { left: c(gt, { title: G ? "Host Details" : "New Host", children: c(pt, { ...u, entry: f, onChangeVault: $ }) }), right: B(ft, { margin: "md", children: [c(Vt, { status: x }), c(mt, { ref: S.triggerRef, onClick: L, disabled: Ct(P), "aria-label": "Three dots button", children: c(St, {}) }), c(bt, { onClick: i })] }), showBorder: true }), c(Ve, { value: f, children: c(Ut, { ...u, isVisible: n, shareSlot: Y, onConnect: z, isOpenedFromEmptyScreen: b, connectLabel: C, shouldShowAllSshConfigs: g, shouldShowAllTelnetConfigs: y, shouldDisableFormFields: !D }) }), c(vt, { onClose: S.closeMenu, autoClose: true, open: S.isOpened, anchorEl: S.triggerRef.current, anchorOrigin: { horizontal: "left", vertical: "bottom" }, transformOrigin: { horizontal: "left", vertical: "top" }, menuItems: P })] }) });
}, Io = (r) => {
  const { onClosed: e, updateSliderAnimationProgress: n, resetSlider: i, isOpened: a, groupForm: b, onCloseSlider: C, connectToHost: v, isOpenedFromEmptyScreen: m, connectLabel: h, inTerminal: g, sliderOptions: p } = r, y = H(xt.host.route);
  return c(kt, { inTerminal: g, isOpened: a, onOpenAnimationStarted: () => n(true), onOpenAnimationEnd: () => n(false), onCloseAnimationStarted: () => n(true), onCloseAnimationEnd: () => {
    n(false), i(), e == null || e();
  }, children: B(Tt, { children: [c(ve, { when: y === "host", children: c(Kt, { onClose: C, isOpened: a, connectToHost: v, isOpenedFromEmptyScreen: m, connectLabel: h }, p == null ? void 0 : p.wizardID) }), c(ve, { when: y === "group", children: c(Et, { onClose: C, isOpened: a, ...b }, p == null ? void 0 : p.wizardID) })] }) });
};
export {
  Io as HostsScreenSidebar
};
