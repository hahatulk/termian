/*
 * Recovered Termius UI module: ChangeVault-5c30bb29.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as y, bt as e, br as r, bu as u, cy as b, dE as K, a6 as j, w3 as U, fB as Z, fb as W, eg as G, lF as z, ml as Y, lc as q, eX as Q, ba as X, aQ as J, bU as ee, e2 as te, mq as oe, gu as ne, fR as re, w4 as ae, w5 as ie, r as i, cv as h, dl as T, dK as P, w6 as E, cY as se, bP as le, jV as ce, w7 as N, bs as V, c5 as B, w8 as H, by as $, w9 as de, wa as _, bq as O, bj as L, wb as pe, wc as me, wd as ue, we as he, wf as ge, b5 as ye, wg as ve, wh as xe, wi as w } from "./reconnectSaga-3b11f62e.js";
import { p as S, k as be } from "./PortForwardingRulePresenter-4942c31a.js";
import { R as Ce, a as R, b as fe } from "./RadioOptionsList-6f3efe10.js";
const we = (t) => {
  const { classes: o } = ke();
  return e("div", { className: o.root, children: e("div", { className: o.body, children: t.children }) });
}, ke = y()({ root: { width: "720px", height: "540px", "@media (max-height: 540px)": { height: "440px" }, position: "absolute", overflow: "hidden", top: "50%", left: "50%", transform: "translate(-50%, -50%)", backgroundColor: "var(--light-grey-7)", borderRadius: "20px", "--content-background": "var(--white)", "--text-color": "var(--dark-grey-1)", "--transition-duration": "250ms", "--transition-function": "cubic-bezier(0.4, 0, 0.2, 1)", ".termius-dark-theme &": { backgroundColor: "var(--dark-grey-3)", "--content-background": "var(--dark-grey-4)", "--text-color": "var(--white)" }, color: "var(--text-color)", "@media (max-height: 460px) and (max-width:730px)": { borderRadius: 0 } }, body: { position: "relative", zIndex: 1, height: "100%", overflow: "hidden" } }), Se = () => {
  const { classes: t } = Me();
  return e("div", { className: t.root, children: r("div", { className: t.body, children: [e(u, { fontSize: 24, bold: true, children: "Processing" }), e(b, { top: 20, children: e(u, { variant: "body1", children: "We need a few moments..." }) }), r("div", { className: t.spinner, children: [e(K, { size: 205, value: 100, className: t.progressDefault, thickness: 2, variant: "determinate" }), e(K, { size: 205, value: 80, className: t.progress, thickness: 2, variant: "indeterminate" })] })] }) });
}, Me = y()({ root: { padding: "40px", "@media (max-height: 540px)": { paddingTop: "20px" } }, body: { width: 523, display: "flex", flexDirection: "column", alignItems: "center", textAlign: "center", margin: "auto" }, progress: { position: "absolute", color: "var(--accent)" }, progressDefault: { position: "absolute", color: "var(--light-grey-4)", ".termius-dark-theme &": { color: "var(--dark-grey-4)" } }, spinner: { marginTop: "104px", width: "205px", height: "205px", position: "relative", display: "flex", "@media (max-height: 540px)": { marginTop: "52px" } } });
function Ne(t) {
  switch (j(U.is(t), "non entity passed"), t.entityName) {
    case "host":
      return new X(t);
    case "group":
      return new Q(t);
    case "snippet":
      return new q(t);
    case "snippetsPackage":
      return new Y(t);
    case "portForwardingRule":
      return new z(t);
    case "identity":
      return new G(t);
    case "sshKey":
      return new W(t);
    case "knownHost":
      return new Z(t);
  }
  throw new J(`Cannot un-marshall ${t.entityName}`);
}
const _e = { host: ee, group: te, portForwardingRule: S, snippetsPackage: oe, snippet: ne, identity: re, sshKey: ae, knownHost: be }, k = (t) => {
  const o = Ne(t), a = Re(o);
  return { icon: a.icon(o), label: a.label(o), description: a.description(o) };
};
function Re(t) {
  return _e[t.entityName];
}
function Pe(t) {
  var s;
  const o = t.length === 1 ? "singular" : "plural", a = ((s = t[0]) == null ? void 0 : s.entityName) ?? "default", d = t.every((n) => n.entityName === a);
  return ie[d ? a : "default"][o];
}
const F = (t) => {
  const { className: o, children: a } = t, { classes: d, cx: s } = Ee();
  return e("div", { className: s(d.footer, o), children: a });
}, Ee = y()({ footer: { "--border-color": "var(--light-grey-5)", ".termius-dark-theme &": { "--border-color": "var(--dark-grey-4)" }, padding: "18.5px 20px", display: "flex", justifyContent: "space-between", borderTop: "1px solid var(--border-color)" } }), Le = ["host", "snippet", "identity", "sshKey"], Te = (t) => {
  const { backReferences: o, onCopy: a, onMove: d, onCancel: s } = t, { classes: n } = Ae(), [c, p] = i.useState({ host: true, snippet: true, identity: true, sshKey: true });
  return r("div", { className: n.root, children: [r("div", { className: n.body, children: [e(u, { align: "center", fontSize: 24, bold: true, children: "Copy or move dependencies?" }), e(b, { top: 20, bottom: 20, children: e(u, { align: "center", variant: "body1", children: "There are dependencies which are used in other entities and moving them will break their usage. Move or copy the dependencies below?" }) }), e("div", { className: n.container, children: Le.map((m) => {
    var g;
    return ((g = o[m]) == null ? void 0 : g.length) > 0 && e(Fe, { entityName: m, backReferences: o, onCollapse: () => p({ ...c, [m]: !c[m] }), collapsed: c[m] }, m);
  }) })] }), r(F, { children: [e(h, { color: "regular", variant: "text", onClick: s, children: "Cancel" }), r(T, { children: [e(h, { color: "accent", variant: "text", onClick: d, children: "Move" }), e(h, { color: "accent", size: "medium", type: "button", onClick: a, children: "Copy" })] })] })] });
}, Fe = (t) => {
  const { backReferences: o, entityName: a, onCollapse: d, collapsed: s } = t, { classes: n } = De(), c = o[a];
  return c.length === 0 ? null : r(i.Fragment, { children: [e(b, { bottom: 10, children: e("div", { onClick: d, children: e(P, { className: n.entity, icon: e(E, { className: n.icon, icon: k(c[0]).icon }), title: r(i.Fragment, { children: [c.length, " ", Pe(c)] }), endAdornment: e(se, { className: le(n.arrowIcon, { [n.open]: !s }) }) }) }) }), e(ce, { in: !s, children: c.map((p) => e(b, { left: 20, bottom: 10, children: e(P, { className: n.entity, icon: e(E, { className: n.icon, icon: k(p).icon }), title: k(p).label, subtitle: e("div", { className: n.entitySubtitle, children: e("span", { className: n.entityDescription, children: k(p).description }) }) }) }, `${p.entityName}-${p.local_id}`)) })] });
}, Ae = y()({ root: { display: "flex", flexDirection: "column", justifyContent: "space-between", height: "100%" }, body: { padding: "40px 120px 0px", "@media (max-height: 540px)": { paddingTop: "20px" } }, container: { maxHeight: "305px", "@media (max-height: 540px)": { maxHeight: "205px" }, overflow: "hidden auto", borderRadius: "10px", "&::-webkit-scrollbar": { display: "none" } } }), De = y()({ entity: { color: "var(--text-color)", padding: 10, borderRadius: 14, gap: 10, backgroundColor: "var(--light-grey-6)", ".termius-dark-theme &": { backgroundColor: "var(--dark-grey-4)" } }, entitySubtitle: { display: "flex", alignItems: "center", gap: 10, color: "var(--light-grey-2)", ".termius-dark-theme &": { color: "var(--light-grey-4)" } }, entityDescription: { flex: "1" }, icon: { width: 43, height: 43 }, arrowIcon: { color: "var(--secondary)", fill: "currentColor", width: 10, height: 6, marginTop: "2px", marginRight: "15px", transition: "transform 0.2s" }, open: { transform: "rotate(180deg)" } }), Ie = (t) => {
  const { portForwardingRules: o, onRemove: a, onMove: d, onCancel: s } = t, { classes: n } = Ke();
  return r("div", { className: n.root, children: [r("div", { className: n.body, children: [e(u, { align: "center", fontSize: 24, bold: true, children: "Move or remove port forwarding?" }), e(b, { top: 20, bottom: 20, children: r(u, { align: "center", variant: "body1", children: ["Hosts are used in the following port forwarding rules.", e("br", {}), "Move the rules to the same vault as well?"] }) }), e("div", { className: n.container, children: o.map((c) => e(b, { bottom: 20, children: e(P, { className: n.entity, icon: e(E, { className: n.icon, icon: S.icon(c) }), title: S.label(c), subtitle: e("div", { className: n.entitySubtitle, children: e("span", { className: n.entityDescription, children: S.description(new z(c)) }) }) }) }, String(c.local_id))) })] }), r(F, { children: [e(h, { color: "regular", variant: "text", onClick: s, children: "Cancel" }), r(T, { children: [e(h, { color: "accent", variant: "text", onClick: a, children: "Remove" }), e(h, { color: "accent", size: "medium", type: "button", onClick: d, children: "Move" })] })] })] });
}, Ke = y()({ root: { display: "flex", flexDirection: "column", justifyContent: "space-between", height: "100%" }, body: { padding: "40px 120px 0px", "@media (max-height: 540px)": { paddingTop: "20px" } }, container: { maxHeight: "305px", "@media (max-height: 540px)": { maxHeight: "205px" }, overflow: "hidden auto", borderRadius: "10px", "&::-webkit-scrollbar": { display: "none" } }, entity: { color: "var(--text-color)", padding: 10, borderRadius: 14, gap: 10, backgroundColor: "var(--light-grey-6)", ".termius-dark-theme &": { backgroundColor: "var(--dark-grey-4)" } }, entitySubtitle: { display: "flex", alignItems: "center", gap: 10, color: "var(--light-grey-2)", ".termius-dark-theme &": { color: "var(--light-grey-4)" } }, entityDescription: { flex: "1" }, icon: { width: 43, height: 43, backgroundColor: "var(--dark-blue-solid)", ".termius-dark-theme &": { backgroundColor: "var(--dark-grey-5)" } } }), Ve = { move: "moving to", copy: "copying to", create: "creating at", import: "moving to" }, Be = (t) => {
  const { vault: o, value: a, onChange: d, operation: s, onSubmit: n, onCancel: c, onBack: p = () => {
  }, multiKeyPromo: m, isMultikeyAvailable: g } = t, { classes: l } = He();
  return r("form", { className: l.root, onSubmit: (v) => {
    v.preventDefault(), n({ credentialsMode: a.mode });
  }, children: [r("div", { className: l.body, children: [e(u, { align: "center", fontSize: 24, bold: true, children: "Choose where to store credentials" }), e("div", { className: l.subtitle, children: r(u, { align: "center", variant: "body1", children: ["Select how your teammates will access the items", " ", Ve[s], e("br", {}), r(u, { ellipsis: true, align: "center", className: l.vaultName, children: [N(o), "."] })] }) }), r(Ce, { "aria-label": "Credentials mode", onChange: (v) => {
    d(v.target.value);
  }, value: a.mode, children: [e(R, { id: "no_credentials_sharing", value: "no_credentials_sharing", label: r(V, { direction: "row", alignItems: "center", gap: "5px", children: [e(u, { variant: "subtitle1", children: N(B, { excludeSuffixForPredefinedVaults: true }) }), e(H, { vault: B, variant: "selectCredentialsDialog" })] }), startAdornment: e($, { className: l.optionIcon }), classes: { startAdornment: l.optionStartAdornment }, children: "Your credentials are not shared. Vault members connect with credentials from their personal vaults." }), e(R, { id: "credentials_sharing", value: "credentials_sharing", label: r(V, { direction: "row", alignItems: "center", gap: "5px", maxWidth: "400px", children: [e(u, { variant: "subtitle1", ellipsis: true, children: N(o, { excludeSuffixForPredefinedVaults: true }) }), e(H, { vault: o, variant: "selectCredentialsDialog" })] }), startAdornment: e($, { className: l.optionIcon }), classes: { startAdornment: l.optionStartAdornment }, children: o.isDefault ? "Your credentials are shared. The entire team can connect with credentials from the Team vault." : "Your credentials are shared. Vault members connect with credentials you move to this vault." }), g && e(fe, {}, "separator-1"), g && e(R, { id: "multikey", value: "multikey", label: "MultiKey", startAdornment: e(de, { className: l.optionIcon }), classes: { startAdornment: l.optionStartAdornment }, children: "Vault members connect with personal unextractable passkeys generated on each device." })] })] }), r(F, { className: l.footer, children: [s === "import" ? e("div", {}) : e(h, { color: "regular", variant: "text", onClick: c, children: "Cancel" }), r(T, { children: [s === "import" && r(i.Fragment, { children: [e(h, { onClick: p, size: "medium", children: "Back" }), e(h, { color: "accent", size: "medium", type: "submit", children: a.mode === "multikey" && m ? "Learn More" : "Done" })] }), s !== "import" && r(h, { color: "accent", size: "medium", type: "submit", children: [a.mode === "no_credentials_sharing" && r(i.Fragment, { children: [_(s), " without credentials"] }), a.mode === "credentials_sharing" && r(i.Fragment, { children: [_(s), " with credentials"] }), a.mode === "multikey" && (m ? "Learn More" : r(i.Fragment, { children: [_(s), " and use MultiKey"] }))] })] })] })] });
}, He = y()({ root: { display: "flex", flexDirection: "column", justifyContent: "space-between", height: "100%" }, optionStartAdornment: { margin: "auto" }, optionIcon: { width: "16px", height: "16px" }, body: { padding: "40px 40px 0px", "@media (max-height: 540px)": { paddingTop: "15px", paddingBottom: "15px" } }, footer: { "@media (max-height: 540px)": { padding: "8px 20px" } }, subtitle: { marginTop: "10px", marginBottom: "30px", "@media (max-height: 540px)": { marginTop: "10px", marginBottom: "12px" } }, vaultName: { display: "inline-block", lineHeight: "normal", verticalAlign: "text-bottom", maxWidth: "320px", "&:before": { content: '" "', whiteSpace: "pre" }, "&:after": { content: '" "', whiteSpace: "pre" } } }), $e = (t, o) => i.createElement("svg", { width: 297, height: 133, viewBox: "0 0 297 133", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: o, ...t }, i.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M66.2033 0.554688C75.542 0.554688 83.9363 4.02482 89.7316 9.54472C96.8106 3.96058 106.232 0.554688 116.574 0.554688C138.565 0.554688 156.392 15.9536 156.392 34.9491C156.392 35.866 156.35 36.7745 156.268 37.6733C169.935 43.1218 179.419 55.0635 179.419 68.9186C179.419 87.9141 161.592 103.313 139.601 103.313C127.55 103.313 116.749 98.6883 109.447 91.3812C102.48 99.4267 89.8016 103.313 75.3167 103.313C56.8139 103.313 41.2588 96.9718 36.7899 83.7201C36.0463 83.7607 35.2963 83.7812 34.5406 83.7812C15.4644 83.7812 0 70.6637 0 54.4823C0 38.3009 15.4644 25.1833 34.5406 25.1833L34.6783 25.1835C36.1743 11.369 49.7174 0.554688 66.2033 0.554688Z", fill: "url(#paint0_linear_3389_9525)" }), i.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M199.156 82.8086C199.156 108.3 178.491 128.965 153 128.965C127.509 128.965 106.844 108.3 106.844 82.8086C106.844 57.3173 127.509 36.6525 153 36.6525C178.491 36.6525 199.156 57.3173 199.156 82.8086ZM203 82.8086C203 110.423 180.614 132.809 153 132.809C125.386 132.809 103 110.423 103 82.8086C103 55.1944 125.386 32.8086 153 32.8086C180.614 32.8086 203 55.1944 203 82.8086ZM146.412 91.1637L169.816 67.7592L173.088 71.0316L148.048 96.0722L146.412 97.6373L144.775 96.0722L133.393 84.6901L136.666 81.4178L146.412 91.1637Z", fill: "#2091F6" }), i.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M199.156 82.8086C199.156 108.3 178.491 128.965 153 128.965C127.509 128.965 106.844 108.3 106.844 82.8086C106.844 57.3173 127.509 36.6525 153 36.6525C178.491 36.6525 199.156 57.3173 199.156 82.8086ZM203 82.8086C203 110.423 180.614 132.809 153 132.809C125.386 132.809 103 110.423 103 82.8086C103 55.1944 125.386 32.8086 153 32.8086C180.614 32.8086 203 55.1944 203 82.8086ZM146.412 91.1637L169.816 67.7592L173.088 71.0316L148.048 96.0722L146.412 97.6373L144.775 96.0722L133.393 84.6901L136.666 81.4178L146.412 91.1637Z", fill: "url(#paint1_linear_3389_9525)" }), i.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M211.018 45.9121C218.073 45.9121 224.414 48.5334 228.791 52.703C234.139 48.4849 241.255 45.9121 249.068 45.9121C265.679 45.9121 279.146 57.5444 279.146 71.8935C279.146 72.5861 279.115 73.2724 279.053 73.9514C289.377 78.067 296.541 87.0879 296.541 97.5542C296.541 111.903 283.075 123.536 266.463 123.536C257.359 123.536 249.2 120.042 243.685 114.522C238.422 120.6 228.844 123.536 217.902 123.536C203.925 123.536 192.174 118.745 188.799 108.734C188.237 108.765 187.671 108.78 187.1 108.78C172.69 108.78 161.008 98.8714 161.008 86.6481C161.008 74.4248 172.69 64.5158 187.1 64.5158L187.205 64.5159C188.335 54.0809 198.565 45.9121 211.018 45.9121Z", fill: "url(#paint2_linear_3389_9525)" }), i.createElement("defs", null, i.createElement("linearGradient", { id: "paint0_linear_3389_9525", x1: 53.0481, y1: 37.2117, x2: 53.0481, y2: 79.2058, gradientUnits: "userSpaceOnUse" }, i.createElement("stop", { stopColor: "currentColor" }), i.createElement("stop", { offset: 1, stopColor: "currentColor", stopOpacity: 0 })), i.createElement("linearGradient", { id: "paint1_linear_3389_9525", x1: 103, y1: 132.809, x2: 188.224, y2: 21.4957, gradientUnits: "userSpaceOnUse" }, i.createElement("stop", { stopColor: "#2091F6" }), i.createElement("stop", { offset: 1, stopColor: "#21B568" })), i.createElement("linearGradient", { id: "paint2_linear_3389_9525", x1: 201.08, y1: 73.6028, x2: 201.08, y2: 105.325, gradientUnits: "userSpaceOnUse" }, i.createElement("stop", { stopColor: "currentColor" }), i.createElement("stop", { offset: 1, stopColor: "currentColor", stopOpacity: 0 })))), ze = i.forwardRef($e), Oe = i.memo(ze), je = () => {
  const { classes: t } = Ue();
  return r("div", { className: t.root, children: [e(u, { fontSize: 24, bold: true, align: "center", children: "Successfully completed!" }), e(Oe, { className: t.successImage })] });
}, Ue = y()({ root: { padding: "40px", display: "flex", flexDirection: "column", justifyContent: "center", alignItems: "center", gap: 35, fontSize: 14, lineHeight: 1.3, "@media (max-height: 540px)": { paddingTop: "20px" } }, successImage: { marginTop: 60, color: "var(--light-grey-5)", ".termius-dark-theme &": { color: "var(--dark-grey-4)" } } }), Ze = ({ onSubmit: t, wizardID: o }) => {
  const a = O(), d = L(pe), s = L(me), [n, c] = i.useState("no_credentials_sharing"), p = (l) => {
    a(ue({ credentialsSharingMode: he[l], wizardID: o })), c(l);
  }, m = (l) => {
    if (n === "multikey" && d) {
      g();
      return;
    }
    t(l.credentialsMode);
  }, g = () => {
    a(ge()), ye({ title: "Learn more about MultiKey", url: "https://termi.us/multikey-buy" });
  };
  return { changeCredentialsMode: p, chooseCredentialsMode: m, credentialsMode: n, multiKeyPromo: d, isMultikeyAvailable: s };
}, We = ({ wizardID: t, operation: o = "move", closeDialog: a }) => {
  const d = O(), s = (x) => d(w.select_credentials_mode({ wizardID: t, windowId: f, payload: x })), { changeCredentialsMode: n, chooseCredentialsMode: c, credentialsMode: p, multiKeyPromo: m, isMultikeyAvailable: g } = Ze({ onSubmit: s, wizardID: t }), l = L((x) => ve(x, t)), v = xe(l == null ? void 0 : l.targetState);
  if (!l || !v) return null;
  const M = () => {
    d(w.closed({ wizardID: t, windowId: f })), a();
  }, A = (x) => d(w.resolve_conflicts({ wizardID: t, windowId: f, payload: x })), D = (x) => d(w.resolve_port_forwarding_conflicts({ wizardID: t, windowId: f, payload: x })), { step: C, windowId: f, backReferences: I } = l;
  return r(we, { children: [C === "select_credentials_mode" && e(Be, { vault: v, operation: o, value: { mode: p }, onChange: n, onSubmit: c, onCancel: M, multiKeyPromo: m, isMultikeyAvailable: g }), C === "resolve_conflicts" && e(Te, { vaultName: v.name, backReferences: I, onCopy: () => A("Copy"), onMove: () => A("Move"), onCancel: M }), C === "resolve_port_forwarding_conflicts" && e(Ie, { portForwardingRules: I.portForwarding, onRemove: () => D("Remove"), onMove: () => D("Move"), onCancel: M }), C === "process" && e(Se, {}), C === "success" && e(je, {})] });
}, Qe = We;
export {
  Qe as default
};
