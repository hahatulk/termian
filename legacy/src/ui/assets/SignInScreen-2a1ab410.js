/*
 * Recovered Termius UI module: SignInScreen-2a1ab410.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as f, jB as y, iD as k, bt as o } from "./reconnectSaga-3b11f62e.js";
import { F as v } from "./FullScreenDialogLayout-e1c06c79.js";
import { A as e } from "./AnimationHandler-1e0795eb.js";
import { A as C } from "./AuthorizationForm-253327c1.js";
import { t as b } from "./transitionScreenWrapper-62515248.js";
import "./OTPToken-a5994a9f.js";
import "./InputDialog-b3f32c9f.js";
import "./AlertDialog-b50b9ed7.js";
import "./Button-90700c84.js";
import "./PasswordInput-9d57733c.js";
import "./OtpLogin-e15265bf.js";
const S = (i) => {
  const { classes: r } = x(), { mounted: n, opacity: a, translate: s, opacityContainer: l, onTransitionEndContainer: m, contentRef: d, containerRef: c, isAnimationInProgress: p, closeDialog: u, initEmail: g } = i, t = { mounted: n, opacity: a, translate: s, opacityContainer: l, onTransitionEndContainer: m, contentRef: d, containerRef: c, isAnimationInProgress: p }, h = () => {
    u();
  };
  return o(e, { animationProps: t, isContainer: true, children: o("div", { className: r.root, children: o(v, { fixedFooter: true, hideOverflow: true, center: o(e, { order: 1, animationProps: t, isLastElement: true, children: o("div", { className: r.content, children: o(C, { isReLogin: true, defaultAuthorizationType: "re-login", from: "Settings", initialEmail: g, onLoggedIn: h }) }) }) }) }) });
}, x = f()({ ...y, root: { height: "100%", overflow: "hidden", backgroundColor: "var(--light-grey-6)", ".termius-dark-theme &": { backgroundColor: "var(--dark-grey-2)" } }, content: { width: "470px", padding: "30px", borderRadius: "23px", backgroundColor: "var(--white)", ".termius-dark-theme &": { backgroundColor: "var(--dark-grey-3)" } } }), P = k(b(S));
export {
  P as default
};
