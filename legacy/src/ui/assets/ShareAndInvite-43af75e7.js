/*
 * Recovered Termius UI module: ShareAndInvite-43af75e7.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bj as _, gG as z, bX as $, ym as ie, br as a, bs as r, bt as e, b$ as y, c0 as N, c1 as L, cX as re, bi as k, r as S, z2 as se, cv as T, bu as v, vI as oe, g4 as K, cC as R, cY as j, jV as W, dl as Y, e0 as ce, aY as le, xK as U, bq as de, cN as he, r8 as me, qo as pe, z3 as ue, bA as M, ew as G, y0 as ge, bC as ve, z4 as xe, f3 as Se, pm as fe, bP as Ce, xH as b, xj as be, bz as Ae, x$ as Ie } from "./reconnectSaga-3b11f62e.js";
import { M as ye, A as Te } from "./AuthorizationForm-253327c1.js";
import { R as _e, a as V } from "./RadioOptionsList-6f3efe10.js";
import { r as we } from "./removeElementByIndex-6c99a5b8.js";
import { S as P, a as Oe } from "./applyTeamVaultTargetState-377d7128.js";
import { S as De } from "./SuccessScreen-cc065469.js";
import { u as Ee } from "./fadeIn-d66fb556.js";
import { S as Ne } from "./SuccessStep-750a80f6.js";
import "./OTPToken-a5994a9f.js";
import "./InputDialog-b3f32c9f.js";
import "./AlertDialog-b50b9ed7.js";
import "./Button-90700c84.js";
import "./PasswordInput-9d57733c.js";
import "./OtpLogin-e15265bf.js";
import "./DialogPanel-fd3d3785.js";
const Le = (s) => {
  const { invites: n } = s, c = _(z), d = _($), h = ie(s.invites, 2);
  return a(r, { flexDirection: "row", children: [e(r, { marginRight: "-5px", zIndex: 2, children: e(y, { isActive: true, color: N(c, d ?? 0), letter: L({ email: c }) }) }), e(r, { flexDirection: "row-reverse", children: h.map((t, i) => e(y, { color: N(t.email, i ?? 0), letter: L({ email: t.email }) })) }), n.length > 2 && e(re, { position: "relative", right: "5px", children: e(y, { letter: (n.length + 1).toString() }) })] });
}, He = (s) => {
  const { hasSharingSelector: n = true, numberOfSharedItems: c, onCancel: d, onSubmit: h } = s, { classes: t, cx: i } = Me(), f = S.useRef(null), A = S.useRef(null), m = S.useRef(null), [o, C] = S.useState("credentials"), l = o === "credentials", p = o === "invites", [g, w] = S.useState(s.invites), [x, q] = S.useState(s.credentialsMode), [O, H] = S.useState({ email: "", role: "Editor" }), D = _(z), X = _($), I = g.length !== 0, Z = !!g.length && l, F = () => {
    if (O.email === D) {
      H({ email: "", role: "Editor" });
      return;
    }
    O.email.length !== 0 && w(ce([...g, O], "email")), H({ email: "", role: "Editor" });
  }, J = (u) => {
    const { key: E } = u;
    E === "Enter" && (u.preventDefault(), F());
  }, Q = (u) => {
    H({ email: u.email, role: "Editor" });
  }, ee = () => n ? I ? !x && l : !x || p : !I, te = () => !n || !x && l && I ? "Done" : !x || !I && l ? "Next" : "Done", ne = () => {
    if (!n) {
      h(g, "no_credentials_sharing");
      return;
    }
    if (x && !I && l) {
      C("invites");
      return;
    }
    if (!x && p && I) {
      C("credentials");
      return;
    }
    h(g, x);
  }, B = a(r, { padding: "0px 20px", bgcolor: n ? "var(--background)" : "var(--foreground)", borderRadius: g.length ? "0px" : "0px 0px 10px 10px", children: [a(r, { flexDirection: "row", width: "100%", gap: "10px", paddingBottom: "20px", ref: f, children: [e(se, { onKeyUp: J, email: O.email, onChange: Q }), e(T, { onClick: F, color: "accent", children: "Add" })] }), e(v, { variant: "subtitle1", color: "main", children: "People with access to the team vault" }), e("div", { className: i(t.teamListWrapper, { [t.increasedHeight]: !n, [t.stretchableHeight]: g.length > 1 && n, [t.teamListHasCredentialsMode]: n }), children: a(r, { gap: "6px", marginTop: "2px", paddingBottom: "10px", children: [e(r, { flexDirection: "row", alignItems: "center", justifyContent: "space-between", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", width: "100%", height: "30px", className: t.item, children: a(r, { direction: "row", alignItems: "center", children: [e(y, { isActive: true, color: N(D, X ?? 0), letter: L({ email: D }) }), e(v, { className: t.memberTitle, color: "main", fontSize: 14, children: D })] }) }), e(r, { gap: "6px", ref: A, children: g.map((u, E) => a(r, { flexDirection: "row", alignItems: "center", justifyContent: "space-between", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", width: "100%", height: "30px", className: t.item, children: [a(r, { direction: "row", alignItems: "center", children: [e(y, { color: N(u.email, E ?? 0), letter: L({ email: u.email }) }), e(v, { className: t.memberTitle, color: "main", fontSize: 14, ellipsis: true, children: u.email })] }), e(r, { onClick: () => w(we(g, E)), children: e(oe, { width: "14px", height: "14px", className: t.removeIcon }) })] }, u.email)) })] }) })] }), ae = S.useRef(null);
  return K(() => {
    var u;
    (u = ae.current) == null || u.focus();
  }), a(r, { ref: m, justifyContent: "space-between", height: "100%", className: i({ [t.rootHasCredentialsMode]: n, [t.rootNoCredentialsMode]: !n }), children: [a(r, { maxHeight: "466px", paddingTop: "40px", className: i({ [t.mainPartHasCredentialsMode]: n, [t.mainPartNoCredentialsMode]: !n }), children: [e(v, { variant: "h2", color: "main", align: "center", children: `Move ${R("item", c, true)} to the Team Vault` }), a(r, { justifyContent: "space-between", padding: "20px 40px 40px 40px", gap: "20px", className: i({ [t.stepsWrapperHasCredentialsMode]: n, [t.stepsWrapperNoCredentialsMode]: !n }), children: [n && a("div", { className: i({ [t.activeStep]: l }), children: [a("div", { className: i(t.step, { [t.noBorder]: l, [t.onHover]: p }), onClick: () => {
    C(l ? "invites" : "credentials");
  }, children: [a(r, { flexDirection: "row", alignItems: "center", children: [a(v, { variant: "body1", color: "main", display: "inline", children: [a(v, { bold: true, display: "inline", children: ["Step 1:", " "] }), "select what access is used in your team"] }), " ", x && p && e(v, { variant: "body1", color: "secondary", children: x === "credentials_sharing" ? " - shared credentials" : " - private credentials" })] }), e(j, { className: i(t.chevron, { [t.chevronOpened]: l }) })] }), e(W, { in: l, children: a(_e, { className: t.optionsList, "aria-label": "Credentials mode", onChange: (u) => {
    q(u.target.value);
  }, value: x, children: [e(V, { id: "credentials_sharing", value: "credentials_sharing", label: "Members share one set of credentials", children: e(P, { mode: "credentials_sharing", variant: "shareAndInviteDialog" }) }, "credentials_sharing"), e(V, { id: "no_credentials_sharing", value: "no_credentials_sharing", label: "Members use their own credentials", children: e(P, { mode: "no_credentials_sharing", variant: "shareAndInviteDialog" }) }, "no_credentials_sharing")] }) })] }), n && a(r, { gap: "20px", maxHeight: "249px", className: i(t.credentialsSection, { [t.activeStep]: p }), children: [a("div", { className: i(t.step, { [t.noBorder]: p, [t.onHover]: l }), onClick: () => {
    C(p ? "credentials" : "invites");
  }, children: [a(v, { variant: "body1", color: "main", display: "inline", children: [a(v, { bold: true, display: "inline", children: ["Step 2:", " "] }), "invite people to your team"] }), a(r, { flexDirection: "row", alignItems: "center", gap: "20px", children: [Z && e(Le, { invites: g }), e(j, { className: i(t.chevron, { [t.chevronOpened]: p }) })] })] }), e(W, { in: p, children: B })] }), !n && B] })] }), a(Y, { className: i(t.bottomButtons, { [t.bottomButtonsHasCredentialsMode]: n }), children: [e(T, { color: "regular", variant: "text", onClick: d, children: "Cancel" }), e(T, { disabled: ee(), onClick: ne, color: "accent", children: te() })] })] });
}, Me = k()((s, n, c) => ({ item: {}, rootHasCredentialsMode: { "@media (max-height: 530px)": { justifyContent: "space-around" } }, rootNoCredentialsMode: { "@media (max-height: 530px)": { justifyContent: "normal", height: "100vh" } }, mainPartHasCredentialsMode: { "@media (max-height: 530px)": { display: "flex", flexDirection: "column", paddingTop: "4vh", maxHeight: "75vh" }, "@media (max-height: 480px)": { maxHeight: "65vh" } }, mainPartNoCredentialsMode: { "@media (max-height: 480px)": { paddingTop: "65px" } }, stepsWrapperHasCredentialsMode: { "@media (max-height: 480px)": { padding: "20px" } }, stepsWrapperNoCredentialsMode: { padding: "20px 40px 0px " }, teamListWrapper: { width: "615px", minHeight: "106px", maxHeight: "147px", overflowY: "scroll", marginTop: "5px", paddingRight: "15px", transition: "all 0.3s ease" }, optionsList: { padding: "0 20px 20px" }, stretchableHeight: { minHeight: "147px", transition: "all 0.3s ease" }, teamListHasCredentialsMode: { "@media (max-height: 530px)": { maxHeight: "0", minHeight: "24.5vh" }, "@media (max-height: 480px)": { maxHeight: "0", minHeight: "19.5vh" } }, increasedHeight: { maxHeight: "288px", "@media (max-height: 530px)": { maxHeight: "0", minHeight: "54.5vh" }, "@media (max-height: 480px)": { minHeight: "49vh" } }, memberTitle: { marginLeft: "8px", overflow: "hidden", textOverflow: "ellipsis", maxWidth: "493px" }, step: { height: "61px", padding: "20px", display: "flex", alignItems: "center", justifyContent: "space-between", border: "1px solid var(--card-b)", borderRadius: "10px", cursor: "pointer", ".termius-dark-theme &": { borderColor: "var(--card-c)" } }, credentialsSection: { transition: "all 0.25s" }, activeStep: { gap: "0px", background: "var(--background)", transition: "all 0.25s", borderRadius: "10px" }, noBorder: { border: "none" }, onHover: { "&:hover": { background: "var(--card-b)", ".termius-dark-theme &": { background: "var(--card-c)", borderColor: "var(--card-c)" } } }, chevron: { width: "11px", height: "7px", fill: "var(--main)", transition: "transform 0.2s", color: "var(--main)", "&:hover": { color: "var(--accent)", ".termius-dark-theme &": { color: "var(--accent)" } } }, chevronOpened: { transform: "rotate(180deg)" }, bottomButtons: { width: "100%", minHeight: "74px", justifyContent: "space-between", borderTop: "1px solid var(--card-b)", padding: "0 20px", ".termius-dark-theme &": { borderColor: "var(--card-c)" } }, bottomButtonsHasCredentialsMode: { "@media (max-height: 530px)": { minHeight: "14vh" } }, removeIcon: { display: "none", color: "var(--secondary)", cursor: "pointer", "&:hover": { color: "var(--red)" }, [`.${c.item}:hover &`]: { display: "block" } } })), Re = (s) => {
  const { onCloseDialog: n, entitiesToShare: c, entryPointForAnalytics: d, teamTrialFunnelID: h } = s, t = de(), i = he(), f = me(), A = _(z), [m, o, C] = pe(ze, { context: { flowType: A ? "authorized" : "signUp" }, actions: { onDismiss: () => {
    n();
  }, onCloseAllRelatedDialogs: () => {
    t(ue("ShareAndInvite"));
  }, sendSuccessSharingAnalytics: () => {
    t(M("createTeamWizardViewOpened", { createTeamWizardStep: "Success", openedFrom: d, teamTrialFunnelID: h, trial: "yes" }));
  }, sendAnalyticsOnAuthorizationFlowOpened: () => {
    t(M("createTeamWizardViewOpened", { createTeamWizardStep: "Create Account", openedFrom: d, teamTrialFunnelID: h, trial: "yes" }));
  }, sendAnalyticsOnTeamWizardOpened: () => {
    t(M("createTeamWizardViewOpened", { createTeamWizardStep: "Invite Members", openedFrom: d, teamTrialFunnelID: h, trial: "yes" }));
  } }, services: { createTeamAndShareData: async (l) => {
    await G(i, ge, { createTeam: true, teamName: "My Team", force2fa: false, invites: l.invites });
    const p = ve(f.getState());
    await G(i, xe, { entities: c, targetState: Oe(p, l.credentialsMode) });
  } } });
  return [m, o, C];
}, ze = le({ id: "shareAndInviteFlow", initial: "introScreen", predictableActionArguments: true, preserveActionOrder: true, schema: { context: {}, events: {}, services: {} }, tsTypes: {}, context: { flowType: "authorized", credentialsMode: null, invites: [] }, on: { CLOSE_DIALOG: { actions: "onDismiss" } }, states: { introScreen: { on: { GO_TO_AUTHORIZATION_FLOW: { target: "authorizationFlow", cond: "isUnauthorizedFlow" }, HANDLE_CONTINUE_BUTTON_CLICK: [{ cond: "isUnauthorizedFlow", target: "authorizationFlow" }, { target: "shareAndInvite" }] } }, authorizationFlow: { entry: "sendAnalyticsOnAuthorizationFlowOpened", on: { GO_TO_INTRO_SCREEN: { target: "introScreen" }, GO_TO_SHARE_AND_INVITE_FLOW: { target: "shareAndInvite" }, CLOSE_ALL_RELATED_DIALOGS: { actions: "onCloseAllRelatedDialogs" } } }, shareAndInvite: { entry: "sendAnalyticsOnTeamWizardOpened", on: { CHANGE_CREDENTIALS_MODE: { actions: "setCredentialsMode" }, SUBMIT_DATA: { target: "submittingData" }, UPDATE_INVITES: { actions: "setInvites" } } }, errorScreen: { on: { GO_TO_SHARE_AND_INVITE_FLOW: { target: "shareAndInvite" }, SUBMIT_DATA: { target: "submittingData" } } }, submittingData: { tags: "submitting", invoke: { src: "createTeamAndShareData", onDone: [{ actions: "sendSuccessSharingAnalytics", cond: "isUnauthorizedFlow", target: "authorizationSuccessScreen" }, { actions: "sendSuccessSharingAnalytics", target: "sharingSuccessScreen" }], onError: [{ target: "errorScreen" }] } }, authorizationSuccessScreen: { on: { CLOSE_ALL_RELATED_DIALOGS: { actions: "onCloseAllRelatedDialogs" } } }, sharingSuccessScreen: { on: { CLOSE_ALL_RELATED_DIALOGS: { actions: "onCloseAllRelatedDialogs" } } } } }, { guards: { isUnauthorizedFlow: (s) => s.flowType === "signUp" || s.flowType === "logIn" }, actions: { setCredentialsMode: U({ credentialsMode: (s, { data: n }) => n }), setInvites: U({ invites: (s, { data: n }) => n }) } }), ke = (s) => {
  const { onClose: n, numberOfSharedItems: c } = s, d = "Team Vault has been configured", h = `${R("item", c, true)} ${R("have", c, false)} been moved to the team vault successfully`;
  return e(De, { title: d, description: h, onClose: n });
}, Fe = (s) => {
  const { onClose: n, onContinue: c } = s, { classes: d } = Be(), h = { enter: "enter" }, t = { enter: c }, i = S.useRef(null);
  return K(() => {
    var f;
    (f = i.current) == null || f.focus();
  }), e(Se.HotKeys, { className: d.root, keyMap: h, handlers: t, innerRef: i, children: a(r, { width: "100%", height: "100%", justifyContent: "space-between", alignItems: "center", children: [a(r, { height: "100%", alignItems: "center", justifyContent: "center", gap: "20px", children: [a(r, { alignItems: "center", gap: "10px", children: [e(ye, {}), e(v, { variant: "h2", color: "primary", children: "Set up your team vault" })] }), a(v, { align: "center", variant: "body1", color: "secondary", children: ["Securely share data via team vaults to boost your team's productivity.", e("br", {}), "Ensure everyone in your team is always on the same page."] })] }), a(Y, { className: d.buttons, children: [e(T, { color: "regular", variant: "text", onClick: n, children: "Cancel" }), e(T, { onClick: c, color: "accent", children: "Start setup" })] })] }) });
}, Be = k()({ root: { width: "100%", height: "100%", display: "flex", flexDirection: "column", alignItems: "center", "@media (max-height: 520px)": { minHeight: "100vh" } }, buttons: { width: "100%", minHeight: "74px", justifyContent: "space-between", borderTop: "1px solid var(--card-b)", padding: "0 20px", ".termius-dark-theme &": { borderColor: "var(--card-c)" } } }), je = (s) => {
  const { closeDialog: n, hasSharingSelector: c = true, teamTrialFunnelID: d, entryPointForAnalytics: h, ...t } = s, { classes: i } = We(), f = Ee(t), A = Object.values(s.entitiesToShare).flat(), [m, o] = Re({ onCloseDialog: () => {
    n();
  }, entitiesToShare: A, entryPointForAnalytics: h, teamTrialFunnelID: d }), { invites: C, credentialsMode: l } = m.context, p = A.length;
  return e(fe.div, { style: f, children: e(r, { width: "720px", height: "540px", borderRadius: "20px", position: "relative", overflow: "hidden", className: Ce(i.dialog, { [i.mainScreen]: m.matches("shareAndInvite") }), children: a(r, { width: "100%", height: "100%", flex: 1, gap: "20px", justifyContent: "space-between", alignItems: "stretch", bgcolor: "var(--foreground)", children: [e(b, { in: m.matches("introScreen"), justifyContent: "space-between", children: e(Fe, { onClose: () => o("CLOSE_DIALOG"), onContinue: () => o("HANDLE_CONTINUE_BUTTON_CLICK") }) }), e(b, { in: m.matches("authorizationFlow"), justifyContent: "space-between", children: e(Te, { flowVariant: "shareAndInvite", onBackButtonClick: () => o("GO_TO_INTRO_SCREEN"), onSignedUp: () => o("GO_TO_SHARE_AND_INVITE_FLOW"), onLoggedIn: () => o("CLOSE_ALL_RELATED_DIALOGS"), from: "Team Setup", defaultAuthorizationType: "signup-welcome", backButtonLabel: "", actionDescription: "Don't have account yet?", actionButtonText: "Sign up", signUpPasswordTitle: `Set an ${be}`, createAccountScreenType: "Create Team", teamWizardEntryType: h, welcomeFlowFunnelID: d }) }), e(b, { in: m.matches("shareAndInvite"), justifyContent: "space-between", children: e(He, { invites: C, credentialsMode: l, onSubmit: (g, w) => {
    o("UPDATE_INVITES", { data: g }), o("CHANGE_CREDENTIALS_MODE", { data: w }), o("SUBMIT_DATA");
  }, onCancel: () => o("CLOSE_DIALOG"), numberOfSharedItems: p, hasSharingSelector: c }) }), e(b, { in: m.hasTag("submitting"), justifyContent: "space-between", children: e(r, { justifyContent: "center", alignItems: "center", height: "100%", children: e(Ae, { className: i.progressCircle, size: 205, thickness: 2 }) }) }), e(b, { in: m.matches("authorizationSuccessScreen"), justifyContent: "center", children: e(Ne, { type: "sign-up", variant: "shareAndInvite", onComplete: () => o("CLOSE_ALL_RELATED_DIALOGS") }) }), e(b, { in: m.matches("sharingSuccessScreen"), justifyContent: "space-between", top: "40px", className: i.fadeContainerSuccess, children: e(ke, { numberOfSharedItems: p, onClose: () => o("CLOSE_ALL_RELATED_DIALOGS") }) }), e(b, { in: m.matches("errorScreen"), justifyContent: "space-between", children: e(Ie, { hasRetryBtn: true, onBack: () => o("GO_TO_SHARE_AND_INVITE_FLOW"), onRetry: () => o("SUBMIT_DATA", { data: m.context.invites }) }) })] }) }) });
}, We = k()({ dialog: { boxShadow: "0px 5px 15px rgba(0, 0, 0, 0.15)", "@media (max-height: 520px)": { height: "100vh" }, "@media (max-height: 520px) and (max-width:730px)": { borderRadius: "0", width: "100vw" } }, mainScreen: { "@media (max-height: 520px)": { height: "540px" } }, fadeContainerSuccess: { "@media (max-height: 520px)": { top: "20px" } }, progressCircle: { color: "var(--accent)" } }), at = je;
export {
  at as default
};
