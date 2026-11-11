/*
 * Recovered Termius UI module: AuthorizationStep-05da7af3.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bt as t, xj as r } from "./reconnectSaga-3b11f62e.js";
import { A as p } from "./AuthorizationForm-253327c1.js";
import { D as s } from "./DialogPanel-fd3d3785.js";
function A(o) {
  const { variant: n, onBack: a, onSignUp: i, onSignIn: e, createAccountScreenType: c } = o;
  return t(s, { size: { width: 720, height: 540 }, content: t(p, { flowVariant: n, onBackButtonClick: a, onSignedUp: i, onLoggedIn: e, from: n === "backUpAndSync" ? "BackUpAndSync" : "Feature List", defaultAuthorizationType: "signup-welcome", backButtonLabel: "", actionDescription: "Don't have account yet?", actionButtonText: "Sign up", signUpPasswordTitle: `Set an ${r}`, createAccountScreenType: c }) });
}
export {
  A
};
