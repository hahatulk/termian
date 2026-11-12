/*
 * Recovered Termius UI module: CreateAccountDialog-846825e6.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as o, bt as t, fu as c, br as p, dS as u, dT as r } from "./reconnectSaga-3b11f62e.js";
import { A as d } from "./AuthorizationStep-05da7af3.js";
import { S as e } from "./SuccessStep-750a80f6.js";
import { u as m } from "./useWizard-d6695218.js";
import "./AuthorizationForm-253327c1.js";
import "./OTPToken-a5994a9f.js";
import "./InputDialog-b3f32c9f.js";
import "./AlertDialog-b50b9ed7.js";
import "./Button-90700c84.js";
import "./PasswordInput-9d57733c.js";
import "./OtpLogin-e15265bf.js";
import "./DialogPanel-fd3d3785.js";
const x = o.forwardRef(({ closeDialog: i }, a) => {
  const [n, s] = m(["authorization", "success-sign-up", "success-sign-in"]);
  return t(c, { in: true, children: t("div", { ref: a, children: p(u, { children: [t(r, { when: n === "authorization", children: t(d, { variant: "backUpAndSync", onBack: () => i({ authorized: false }), onSignUp: () => s.to("success-sign-up"), onSignIn: () => s.to("success-sign-in") }) }), t(r, { when: n === "success-sign-up", children: t(e, { type: "sign-up", variant: "backUpAndSync", onComplete: () => i({ authorized: true }) }) }), t(r, { when: n === "success-sign-in", children: t(e, { type: "sign-in", variant: "backUpAndSync", onComplete: () => i({ authorized: true }) }) })] }) }) });
});
export {
  x as default
};
