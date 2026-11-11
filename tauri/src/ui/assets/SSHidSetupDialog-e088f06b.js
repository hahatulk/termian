/*
 * Recovered Termius UI module: SSHidSetupDialog-e088f06b.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as t, ft as F, ca as I, mZ as P, bt as e, fu as R, br as S, bs as U, bu as g, cX as f, fv as q, jV as M, k1 as T, cv as D, cN as _, y$ as L, bj as K, yV as V, z0 as W, ew as y, yW as j, yX as G, z1 as Z, dS as N, dT as l, gw as O } from "./reconnectSaga-3b11f62e.js";
import { u as H } from "./useRequest-2d237c57.js";
import { u as X } from "./useWizard-d6695218.js";
import { u as $, a as x } from "./WarnAboutSSHidDeviceCount-09c75630.js";
import { u as J } from "./sshid-logo-colored.react-d3338d2f.js";
import { D as Q, a as Y } from "./DialogPanel-fd3d3785.js";
import { S as ee } from "./SSHidLinkWithUsernamePreview-ce35b750.js";
import { S as se } from "./SettingUpSSHid-6c4a08cf.js";
import { S as re, a as te, b as ae, c as ne } from "./SSHidFinish-e275127b.js";
import "./retry-circular.react-77791386.js";
const oe = (m, o) => t.createElement("svg", { width: 50, height: 50, viewBox: "0 0 50 50", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: o, ...m }, t.createElement("g", { id: "Extra icons/Personal" }, t.createElement("path", { id: "Vector", fillRule: "evenodd", clipRule: "evenodd", d: "M25.0006 21.7855C30.3262 21.7855 34.6434 17.4682 34.6434 12.1426C34.6434 6.81701 30.3262 2.49976 25.0006 2.49976C19.675 2.49976 15.3577 6.81701 15.3577 12.1426C15.3577 17.4682 19.675 21.7855 25.0006 21.7855ZM25 47.4997C29.7624 47.4997 34.1793 46.0201 37.8159 43.4955C41.7866 40.739 42.6469 35.1226 39.442 31.504C35.9088 27.5147 30.748 25 24.9999 25C19.2519 25 14.0912 27.5147 10.5579 31.5039C7.35295 35.1224 8.21323 40.7388 12.1839 43.4954C15.8206 46.0201 20.2375 47.4997 25 47.4997Z", fill: "url(#paint0_linear_1138_25258)" })), t.createElement("defs", null, t.createElement("linearGradient", { id: "paint0_linear_1138_25258", x1: 6.72034, y1: 2.49976, x2: 47.1176, y2: 43.7401, gradientUnits: "userSpaceOnUse" }, t.createElement("stop", { stopColor: "#1D94FF" }), t.createElement("stop", { offset: 1, stopColor: "#AF5FFF" })))), ie = t.forwardRef(oe), ce = t.memo(ie), ue = ce;
function de(m) {
  var n;
  const { onCancel: o, onContinue: i, progress: a, errorMessage: c } = m, s = F({ mode: "onChange", defaultValues: { username: "" } }), u = s.watch("username"), r = (n = s.formState.errors.username) == null ? void 0 : n.message;
  return I(() => {
    c && s.setError("username", { message: c, type: "value" });
  }), P(() => {
    s.setError("username", { message: c, type: "value" });
  }, [c]), P(() => {
    s.clearErrors("username");
  }, [u]), e("form", { onSubmit: s.handleSubmit(i), children: e(Q, { size: { width: 720, height: 540 }, content: e(R, { in: true, timeout: { enter: 300, exit: 300 }, children: S(U, { gap: "10px", alignItems: "center", children: [e(ue, { style: { height: "64px" } }), e(g, { variant: "h2", align: "center", children: "Choose your SSH.id username" }), S(g, { variant: "body1", color: "secondary", align: "center", children: ["This username can be used as a default username", e("br", {}), "for SSH authentication."] }), S(f, { width: "370px", margin: "0 auto", pt: "10px", minHeight: "100px", children: [e(q, { fullWidth: true, placeholder: "username", autoFocus: true, alertIcons: false, disabled: a, error: !!s.formState.errors.username, InputProps: { ...s.register("username", { required: "Username is required" }) } }), e(M, { in: !!r, children: e(T, { status: "error", children: r }) })] }), e(g, { variant: "body1", color: "secondary", align: "center", children: "Easy way to get your public keys from anywhere." }), e(f, { width: "370px", margin: "0 auto", pt: "10px", children: e(f, { width: "100%", borderRadius: "15px", bgcolor: "var(--background)", p: "10px 15px", children: e(ee, { username: u }) }) })] }) }), actions: e(Y, { start: e(D, { type: "button", variant: "text", onClick: o, disabled: a, children: "Cancel" }), end: e(D, { type: "submit", color: "accent", progress: a, disabled: !s.formState.isValid, children: "Continue" }) }) }) });
}
function ve(m) {
  const { closeDialog: o } = m, [i, a] = X(["creating", "setting-up", "ask-biometrics", "completing", "completing-failed", "finish"]), [c, s] = t.useState(), u = _(), r = L(), n = K(V), h = J(n), k = W(), w = $(), E = x({ wizardID: n.wizardID, onStartGenerating: () => {
    a.to("setting-up");
  } }), C = x(n), B = () => {
    !r.isSetupCompleted && r.allDevicesWithBiometry && !r.isDefaultPasskeysTypeBiometric ? a.to("ask-biometrics") : p.run({ useBiometricKey: false });
  }, v = () => {
    h.sendPostponeSSHidSetupButtonPressed(), o();
  }, p = H(async (d) => {
    await y(u, j, { wizard_id: n.wizardID }), d.useBiometricKey && await y(u, G, { type: "ECDSA-SK", wizard_id: n.wizardID });
  }, { manual: true, onBefore: () => {
    a.to("completing");
  }, onError: () => {
    a.to("completing-failed");
  }, onSuccess: () => {
    a.to("finish");
  } }), b = H(async (d) => y(u, Z, { username: d.username, wizard_id: n.wizardID }), { manual: true, onBefore: () => {
    s(void 0);
  }, onError: (d) => {
    s(d.message);
  }, onSuccess: () => {
    E.request.run();
  } }), A = () => {
    h.sendRetrySSHidCreationButtonPressed(), p.refresh();
  }, z = () => {
    r.username && (h.sendSSHidPublicProfileButtonPressed(), O(r.username), o());
  };
  return S(N, { children: [e(l, { when: i === "creating", children: e(de, { onCancel: () => o(), onContinue: b.run, progress: b.loading, errorMessage: c }) }), e(l, { when: i === "setting-up", children: e(se, { onLater: v, onContinue: B, setupState: k, sshidDevices: r.devices, onLogoutDevice: w.request.run, logoutDeviceProgress: w.deviceInProgress, onRegenerateDevicePasskeys: C.request.run, regenerateDevicePasskeysProgress: C.deviceInProgress }) }), e(l, { when: i === "ask-biometrics", children: e(re, { devices: r.devices, onUse: () => p.run({ useBiometricKey: true }), onDoNotUse: () => p.run({ useBiometricKey: false }) }) }), e(l, { when: i === "completing", children: e(te, {}) }), e(l, { when: i === "completing-failed", children: e(ae, { onLater: v, onTryAgain: A }) }), e(l, { when: i === "finish", children: r.username && e(ne, { username: r.username, onDone: () => o(), onPublicProfile: z }) })] });
}
export {
  ve as default
};
