/*
 * Recovered Termius UI module: SSHidSetupDeviceDialog-ae080f30.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { cN as A, ew as d, yY as b, yZ as z, y_ as K, h$ as I, y$ as q, bj as L, yV as R, z0 as T, yW as E, yX as _, br as G, dS as U, bt as t, dT as c, gw as j } from "./reconnectSaga-3b11f62e.js";
import { u as y } from "./useRequest-2d237c57.js";
import { u as x } from "./useWizard-d6695218.js";
import { u as W, a as C } from "./WarnAboutSSHidDeviceCount-09c75630.js";
import { u as N } from "./sshid-logo-colored.react-d3338d2f.js";
import { S as $ } from "./SettingUpSSHid-6c4a08cf.js";
import { S as F, a as M, b as H, c as O } from "./SSHidFinish-e275127b.js";
import "./retry-circular.react-77791386.js";
import "./DialogPanel-fd3d3785.js";
function V(e) {
  const n = A();
  return y(async () => {
    const s = await d(n, b), u = await d(n, z, s);
    await d(n, K, { ca_key: u, wizard_id: e.wizardID });
  }, { manual: true, onBefore: () => {
    var s;
    (s = e == null ? void 0 : e.onRun) == null || s.call(e);
  }, onSuccess: () => {
    var s;
    (s = e == null ? void 0 : e.onSuccess) == null || s.call(e);
  }, onError: () => {
    var s;
    (s = e == null ? void 0 : e.onError) == null || s.call(e);
  } });
}
function ne(e) {
  const { closeDialog: n, isCurrentDeviceLast: s, needToGenerateCA: u } = e, [o, r] = x([...I(u, ["generate-ca", "generate-ca-failed"]), "generate-passkeys", "ask-biometrics", "completing", "completing-failed", "finish"]), g = A(), i = q(), a = L(R), l = N(a), p = T(), f = W(), h = V({ wizardID: a.wizardID, onRun: () => {
    r.to("generate-ca");
  }, onSuccess: () => {
    D.request.run();
  }, onError: () => {
    r.to("generate-ca-failed");
  } }), D = C({ wizardID: a.wizardID, onStartGenerating: () => {
    r.to("generate-passkeys");
  } }), m = C(a);
  y(async () => {
    u ? h.run() : D.request.run();
  }, { loadingDelay: 1e3 });
  const k = () => {
    !i.isSetupCompleted && i.allDevicesWithBiometry && s ? r.to("ask-biometrics") : S.run({ useBiometricKey: false });
  }, w = () => {
    l.sendPostponeSSHidSetupButtonPressed(), n();
  }, S = y(async (P) => {
    await d(g, E, { wizard_id: a.wizardID }), P.useBiometricKey && await d(g, _, { type: "ECDSA-SK", wizard_id: a.wizardID });
  }, { manual: true, defaultParams: [{ useBiometricKey: false }], onBefore: () => {
    r.to("completing");
  }, onError: () => {
    r.to("completing-failed");
  }, onSuccess: () => {
    r.to("finish");
  } }), v = () => {
    l.sendRetrySSHidCreationButtonPressed(), S.refresh();
  }, B = () => {
    i.username && (l.sendSSHidPublicProfileButtonPressed(), j(i.username), n());
  };
  return G(U, { children: [t(c, { when: o === "generate-passkeys", children: t($, { onLater: w, onContinue: k, setupState: p, sshidDevices: i.devices, onLogoutDevice: f.request.run, logoutDeviceProgress: f.deviceInProgress, onRegenerateDevicePasskeys: m.request.run, regenerateDevicePasskeysProgress: m.deviceInProgress }) }), t(c, { when: o === "ask-biometrics", children: t(F, { devices: i.devices, onUse: () => S.run({ useBiometricKey: true }), onDoNotUse: () => S.run({ useBiometricKey: false }) }) }), t(c, { when: o === "generate-ca" || o === "completing", children: t(M, {}) }), t(c, { when: o === "completing-failed", children: t(H, { onLater: w, onTryAgain: v }) }), t(c, { when: o === "generate-ca-failed", children: t(H, { onLater: () => n(), onTryAgain: () => h.run() }) }), t(c, { when: o === "finish", children: i.username && t(O, { username: i.username, onDone: () => n(), isSSHidSetupCompleted: i.isSetupCompleted, onPublicProfile: B }) })] });
}
export {
  ne as default
};
