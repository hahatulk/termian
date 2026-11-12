/*
 * Recovered Termius UI module: LogsSettings-c332b503.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as I, br as l, Ak as P, bs as s, bt as e, bu as u, cY as _, Al as W, Am as A, e7 as B, bx as z, by as $, bz as U, dZ as F, cu as j, bO as w, cD as Z, cN as H, bq as Y, bj as h, Dk as q, bL as G, c4 as X, zy as J, Dr as K, ca as Q, e3 as ee, cv as te, r$ as ae, jV as ne, cx as oe, bA as D, ck as re, yL as ie, az as se, f1 as le, f2 as ce, ew as de, Ds as ue, c_ as pe, wn as ge } from "./reconnectSaga-3b11f62e.js";
import { P as he } from "./PageWrapper-a7d0cf6b.js";
import { u as me, E as T } from "./PromoBanner-aac31425.js";
import "./triggerChoosePlanDialog-b93628a0.js";
const xe = [{ label: "Enabled", value: true }, { label: "Disabled", value: false }], be = ({ value: t, disabled: n, additionalSelectElement: o, onChange: r, renderLabel: p }) => {
  const { classes: a } = ve();
  return l(P, { value: t, disabled: n, onChange: (c) => r(c.target.value === "true"), className: a.select, IconComponent: () => null, renderValue: (c) => l(s, { direction: "row", gap: "5px", alignItems: "center", paddingLeft: "4px", borderRadius: "4px", children: [e(u, { variant: "body1", children: p(c) }), e(_, { className: a.arrowIcon })] }), inputProps: { className: a.selectClassName }, input: e(W, { className: a.selectMemberButton, label: p(t), tabIndex: -1 }), MenuProps: { anchorOrigin: { vertical: "bottom", horizontal: "center" }, transformOrigin: { vertical: "top", horizontal: "right" }, slotProps: { paper: { className: a.paperWrapper }, root: { className: a.selectMenuRoot } } }, children: [xe.map((c, x) => e(A, { value: c.value ? "true" : "false", className: a.optionClassName, children: l(s, { flexDirection: "row", justifyContent: "space-between", alignItems: "center", width: "100%", children: [e(u, { variant: "body1", children: c.label }), c.value === t && e(B, { className: a.checkIcon })] }) }, x)), o] });
}, M = { hoverLight: "var(--light-grey-transparent-10)", hoverDark: "var(--dark-grey-transparent-10)" }, m = { background: M.hoverLight, ".termius-dark-theme &": { "--hover-color": M.hoverDark } }, ve = I()({ select: { color: "var(--main)", "&:hover": { color: "var(--main)" }, "&.Mui-disabled": { color: "var(--secondary)" } }, selectMenuRoot: { "& .MuiList-root": { padding: 0 } }, arrowIcon: { width: "11px", marginRight: "2px", marginTop: "2px", color: "currentColor" }, selectMemberButton: { fontSize: "14px", paddingRight: "0px", "&.MuiOutlinedInput-notchedOutline": { borderWidth: "0px" }, "& fieldset": { border: "none" } }, optionClassName: { color: "var(--main)", padding: "10px", borderRadius: "10px", "&:hover": m, "&.Mui-selected": { background: "transparent", "&:hover": m }, "&.Mui-selected.Mui-focusVisible": m, "&.Mui-focusVisible": m }, paperWrapper: { padding: 5, border: "1px solid var(--light-grey-transparent-10)", margin: "15px 10px 15px 15px", borderRadius: "15px", background: "var(--white)", ".termius-dark-theme &": { background: "var(--foreground)" }, width: "178px" }, selectClassName: { border: "1px solid transparent", color: "inherit", "&.MuiSelect-select": { paddingRight: "4px !important" }, "&.Mui-disabled": { WebkitTextFillColor: "currentColor" }, padding: 0 }, checkIcon: { color: "var(--secondary)", width: "14px", height: "14px", marginLeft: "10px" } }), L = (t) => {
  const { vault: n, variant: o } = t, { classes: r, cx: p } = O();
  return l(s, { direction: "row", alignItems: "center", justifyContent: "space-between", component: "li", className: p(r.root, { [r.compact]: o === "compact" }), children: [l("section", { className: r.leftSection, children: [e(z, { size: 24, icon: e($, { width: "16px", color: "var(--main)" }) }), e(u, { title: n.name, variant: "body1", color: "primary", ellipsis: true, children: n.name }), o === "default" && t.icon && e(s, { padding: "6px 5px 5px 5px", children: t.icon })] }), o === "default" && e("section", { className: r.rightSection, children: e(be, { value: t.setting, disabled: !n.permissions.has("editVaultSettings") || t.isFetching, onChange: (a) => a !== t.setting && t.onToggle(n, a), renderLabel: (a) => e(fe, { enabled: a, isFetching: t.isFetching }) }) })] });
}, fe = ({ enabled: t, isFetching: n }) => {
  const { classes: o } = O();
  return l(s, { gap: "4px", direction: "row", alignItems: "center", children: [n && e(U, { className: o.spinner, size: 12 }), t ? "Enabled" : "Disabled"] });
}, O = I()(() => ({ root: { padding: "10px", borderRadius: "10px", ".termius-dark-theme &": { boxShadow: "none" } }, compact: { background: "var(--foreground)" }, leftSection: { display: "flex", alignItems: "center", overflow: "hidden", gap: "5px", paddingRight: "10px" }, rightSection: { display: "flex", alignItems: "center", flexShrink: 0, position: "relative" }, spinner: { color: "var(--secondary)" } })), ye = ({ onClose: t, onConfirm: n, vault: o }) => {
  const [r, p] = w.useState(false);
  return e(Z, { title: "Confirm disabling logs", buttonColor: "error", cancelButtonTitle: "Cancel", cancelButtonVariant: "text", confirmButtonTitle: "Disable Logs", onOk: async () => {
    try {
      p(true), await n();
    } finally {
      p(false), t();
    }
  }, onClose: t, fetching: r, disabled: r, children: l(s, { gap: "20px", children: [l(u, { children: ["Disabling logs will ", e("strong", { children: "delete the logs " }), "from this vault:"] }), e(L, { vault: o, variant: "compact" })] }) });
}, Se = F.promisify(j(ye)), Te = () => {
  const { classes: t } = we(), n = H(), o = Y(), r = h(q), p = h(G), a = h(X), c = me(), x = h(J), [R, C] = w.useState(void 0), [b, v] = w.useState("");
  K(() => {
    b && v("");
  }, 1e4), Q(() => {
    o(D("logsSettingsScreenOpened"));
  });
  const E = () => {
    o(ie({ createAccountScreenType: "High-Volume Unsynced Logs Popup", upgradeToProFunnelID: se() })), le(n, { dialog: "CreateAccountDialog", target: ce(), options: { closeOnBackdropClick: false }, props: {} });
  }, k = async (i, d) => {
    var N;
    const y = r.get(i);
    if (!y) return;
    C(i);
    try {
      await de(n, ue, { vaultId: i, enabled: d });
    } catch {
      v(`Couldn't ${d ? "enable" : "disable"} logs in ${(N = r.get(i)) == null ? void 0 : N.name}`);
    } finally {
      C(void 0);
    }
    let S = "Shared";
    pe(y) ? S = "Personal" : ge(y) && (S = "Team"), o(D("logsRetentionPolicyChanged", { vaultType: S, logsRetentionPolicy: d ? "Enabled" : "Disabled" }));
  }, V = (i, d) => {
    v(""), d === false ? Se({ onConfirm: async () => {
      await k(i.id, d);
    }, vault: i }).catch(re.swallow) : k(i.id, d);
  }, g = (() => {
    switch (c) {
      case T.UNAUTHORIZED_OVER_SIZE_LIMIT:
        return { title: `You have ${x.size}mb of logs stored locally`, description: "Change log retention policies or create an account to store logs in a cloud vault." };
      default:
        return null;
    }
  })(), f = (() => {
    switch (c) {
      case T.UNAUTHORIZED_OVER_SIZE_LIMIT:
        return { text: "Create account", onClick: E };
      default:
        return null;
    }
  })();
  return e(ee, { fullHeight: true, children: e(he, { children: e(s, { className: t.root, alignItems: "center", gap: "30px", children: l(s, { className: t.vaultsList, component: "ul", children: [e(u, { variant: "boldBody1", color: "main", children: "Logs retention policies" }), e(u, { variant: "body1", children: "Control how logs are stored in your vaults." }), e("div", { className: t.divider }), (g == null ? void 0 : g.title) && l(s, { className: t.promo, children: [l(s, { gap: "5px", children: [e(u, { variant: "body1", bolder: true, children: g.title }), e(u, { variant: "body1", children: g.description })] }), f && e(te, { size: "small", color: "white", onClick: f.onClick, children: e(u, { variant: "mediumBody1", children: f.text }) })] }), e(s, { component: "ul", "aria-label": "Logs settings list", children: p.map((i) => {
    const d = r.get(i.vault_id);
    return d ? e(L, { variant: "default", vault: d, isFetching: R === i.vault_id, setting: i.enabled, onToggle: V, icon: a && e(ae, { color: "var(--secondary)" }) }, i.vault_id) : null;
  }) }), e(ne, { in: !!b, mountOnEnter: true, unmountOnExit: true, children: l(s, { gap: "10px", direction: "row", alignItems: "center", padding: "0 15px", children: [e(s, { flex: 0, children: e(oe, { className: t.warningIcon }) }), e(u, { variant: "body1", color: "error", ellipsis: true, whiteSpace: "pre-wrap", children: b })] }) })] }) }) }) });
}, we = I()({ root: { background: "var(--background)", color: "var(--main)", padding: "30px 20px", width: "100%", height: "100%", overflowY: "auto", overflowX: "hidden" }, vaultsList: { width: "100%", maxWidth: "700px", listStyle: "none", margin: 0, padding: 15, borderRadius: 15, display: "flex", flexDirection: "column", gap: "10px", backgroundColor: "var(--card-a)", "@media only screen and (max-width: 1280px)": { maxWidth: "600px" }, "@media only screen and (max-width: 900px)": { maxWidth: "500px" }, "@media only screen and (max-width: 800px)": { width: "100%", margin: "30px 20px 0 20px" } }, divider: { height: "1px", width: "100%", backgroundColor: "var(--background-entity)" }, promo: { display: "flex", padding: "10px", gap: "15px", borderRadius: "10px", alignItems: "baseline", alignSelf: "stretch", justifyContent: "center", background: "var(--background-entity)", "& > button": { textTransform: "none" } }, warningIcon: { width: "15px", height: "15px", color: "var(--error-medium)" } });
export {
  Te as LogsSettings
};
