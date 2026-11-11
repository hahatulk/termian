/*
 * Recovered Termius UI module: SSHidCompleteSetupDialog-807f937c.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bj as d, gH as y, yU as g, h$ as P, cN as w, yV as H, ew as S, yW as D, yX as b, br as A, dS as B, bt as e, dT as t, gw as C } from "./reconnectSaga-3b11f62e.js";
import { u as k } from "./useRequest-2d237c57.js";
import { u as R } from "./useWizard-d6695218.js";
import { u as U } from "./sshid-logo-colored.react-d3338d2f.js";
import { S as z, a as K, b as T, c as v } from "./SSHidFinish-e275127b.js";
import "./retry-circular.react-77791386.js";
import "./DialogPanel-fd3d3785.js";
function _(m) {
  const { closeDialog: a, shouldAskAboutBiometrics: l = false } = m, s = d(y), h = d(g), [i, o] = R([...P(l, ["ask-biometrics"]), "completing", "completing-failed", "finish"]), u = w(), n = d(H), r = U(n), f = () => {
    r.sendPostponeSSHidSetupButtonPressed(), a();
  }, c = k(async (p) => {
    await S(u, D, { wizard_id: n.wizardID }), p.useBiometricKey && await S(u, b, { type: "ECDSA-SK", wizard_id: n.wizardID });
  }, { manual: l, defaultParams: [{ useBiometricKey: false }], onBefore: () => {
    o.to("completing");
  }, onError: () => {
    o.to("completing-failed");
  }, onSuccess: () => {
    o.to("finish");
  } });
  return A(B, { children: [e(t, { when: i === "ask-biometrics", children: e(z, { devices: h, onUse: () => c.run({ useBiometricKey: true }), onDoNotUse: () => c.run({ useBiometricKey: false }) }) }), e(t, { when: i === "completing", children: e(K, {}) }), e(t, { when: i === "completing-failed", children: e(T, { onLater: f, onTryAgain: () => {
    r.sendRetrySSHidCreationButtonPressed(), c.refresh();
  } }) }), e(t, { when: i === "finish", children: s && e(v, { username: s, onDone: () => a(), onPublicProfile: () => {
    s && (r.sendSSHidPublicProfileButtonPressed(), C(s), a());
  } }) })] });
}
export {
  _ as default
};
