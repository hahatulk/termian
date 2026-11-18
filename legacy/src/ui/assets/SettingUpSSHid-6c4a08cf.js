/*
 * Recovered Termius UI module: SettingUpSSHid-6c4a08cf.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bt as e, fu as b, br as o, bs as n, bu as s, jV as S, dl as v, cv as a, bz as x, bi as C } from "./reconnectSaga-3b11f62e.js";
import { D as y, a as D } from "./DialogPanel-fd3d3785.js";
import { M as k, S as f } from "./sshid-logo-colored.react-d3338d2f.js";
import { W as P } from "./WarnAboutSSHidDeviceCount-09c75630.js";
function z(r) {
  const { onLater: l, onContinue: c, onLogoutDevice: d, logoutDeviceProgress: h, onRegenerateDevicePasskeys: p, regenerateDevicePasskeysProgress: g, sshidDevices: i, setupState: t } = r, { classes: u } = w(), m = i.length > 1;
  return e(y, { size: { width: 720, height: 540 }, slotProps: { content: { className: u.dialogContent } }, content: e(b, { in: true, timeout: { enter: 300, exit: 300 }, children: o(n, { height: "100%", children: [o(n, { gap: "10px", pb: "10px", alignItems: "center", flexShrink: 0, children: [e(k, {}), e(s, { variant: "h2", align: "center", children: "Passkeys generation" }), m && !t.ableToCompleteNow && e(s, { variant: "body1", color: "secondary", align: "center", children: "Open Termius on other devices or log out." })] }), t.state !== "idle" && e(f, { sshidDevices: i, onLogoutDevice: d, logoutDeviceProgress: h, onRegenerateDevicePasskeys: p, regenerateDevicePasskeysProgress: g }), e(n, { flexShrink: 0, children: e(S, { in: t.showSetupWarning, children: e(n, { pb: "10px", children: e(P, {}) }) }) })] }) }), actions: e(D, { end: o(v, { children: [t.ableToCompleteLater && e(a, { type: "button", onClick: l, children: "Finish later" }), e(a, { type: "button", color: "accent", onClick: c, disabled: !t.ableToCompleteNow, children: t.state === "waiting-other-devices" ? o(n, { gap: "6px", direction: "row", alignItems: "center", children: [e(x, { size: "12px", thickness: 4, color: "inherit" }), "Waiting"] }) : "Continue" })] }) }) });
}
const w = C()({ dialogContent: { paddingBottom: "0px" } });
export {
  z as S
};
