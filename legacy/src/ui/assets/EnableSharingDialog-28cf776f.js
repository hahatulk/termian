/*
 * Recovered Termius UI module: EnableSharingDialog-28cf776f.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as P, cN as N, ft as O, r as D, ew as R, xq as T, br as l, bO as I, bu as S, xj as y, bt as e, wa as j, cv as E, xk as A, bP as B, bv as L, cy as M, iD as W, bq as q, g4 as Y, xr as p, ay as $, bj as G, gO as _ } from "./reconnectSaga-3b11f62e.js";
import { t as z } from "./transitionScreenWrapper-62515248.js";
import { F as H } from "./PasswordInput-5b1b9dd4.js";
import { F as V } from "./FullScreenDialogLayout-e1c06c79.js";
import { M as J } from "./passwordPrompt.react-d6fafff3.js";
import { A as f } from "./AnimationHandler-1e0795eb.js";
import "./PasswordInput-9d57733c.js";
const K = (c) => {
  var h, C;
  const { onSubmit: a, onSuccess: s, onFail: o, isOwner: d } = c, n = N(), { classes: u } = Q(), t = O({ mode: "onChange" }), { register: w, handleSubmit: m, formState: i, setError: r } = t, x = D.useCallback(async (v) => {
    a();
    try {
      await R(n, T, v.password), s();
    } catch (F) {
      r("password", { type: "notMatch", message: F }), o();
    }
  }, [n, a, s, o, r]), { isValid: g, isSubmitting: b } = i;
  return l(I.Fragment, { children: [l(S, { color: "primary", variant: "h2", children: ["Re-enter your ", y] }), d ? l(S, { color: "primary", variant: "body2", children: ["Now you can share groups with your team to make sure your team is up-to-date with the latest changes in the configuration.", e("br", {}), e("br", {}), "To enable this feature, please enter your Termius", " ", y, ".", e("br", {}), e("br", {}), "You can also do it later in settings."] }) : l(S, { color: "primary", variant: "body2", children: ["Now the team administrator can share groups with your team to make sure your team is up-to-date with the latest changes in the configuration.", e("br", {}), e("br", {}), "To enable access to the shared data, please enter your Termius", " ", y, ".", e("br", {}), e("br", {}), "You can also do it later in settings."] }), l("form", { className: u.form, onSubmit: m(x), children: [e(H, { label: `${j(y)}`, autoFocus: true, error: !!i.errors.password, InputProps: w("password", { required: true }) }), e(E, { fullWidth: true, color: "accent", type: "submit", disabled: !g || b, progress: b, children: "Enable group sharing" }), !!((C = (h = i.errors) == null ? void 0 : h.password) != null && C.message) && e(A, { onLeft: true, children: i.errors.password.message })] })] });
}, Q = P()({ form: { flex: 1, display: "flex", flexDirection: "column" } }), k = (c) => {
  const { animationProps: a, children: s, title: o, onPostpone: d, isCloseButtonDisabled: n, actions: u } = c, { classes: t } = U();
  return e(f, { animationProps: a, isContainer: true, children: e("div", { className: B(t.root, "termius-dark-theme"), children: e(V, { top: e(f, { order: 1, animationProps: a, children: e(S, { color: "primary", align: "center", variant: "h2", children: o }) }), left: e(f, { order: 2, animationProps: a, children: e("div", { className: t.imageContainer, children: e(J, {}) }) }), right: e(f, { order: 2, animationProps: a, children: e("div", { className: t.cardContainer, children: l("div", { children: [e(L, { className: t.card, dark: true, children: s }), u || e(M, { top: 20, children: e(E, { onClick: d, disabled: n, className: t.button, children: "Do it later" }) })] }) }) }) }) }) });
}, U = P()({ root: { height: "100%", width: "100%", padding: "40px", paddingBottom: "0", backgroundColor: "var(--dark-grey-1)" }, imageContainer: { width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center", "@media only screen and (max-width: 900px)": { display: "none" } }, cardContainer: { display: "flex", flexDirection: "column", justifyContent: "center", alignItems: "center", position: "relative" }, card: { width: "440px", maxWidth: "100%" }, button: { width: "207px" } });
k.defaultProps = { isCloseButtonDisabled: false, actions: null };
const X = (c) => {
  const { mounted: a, opacity: s, translate: o, opacityContainer: d, onTransitionEndContainer: n, contentRef: u, containerRef: t, isAnimationInProgress: w, closeDialog: m } = c, i = { mounted: a, opacity: s, translate: o, opacityContainer: d, onTransitionEndContainer: n, contentRef: u, containerRef: t, isAnimationInProgress: w }, r = q(), [x, g] = D.useState(false);
  Y(() => {
    r(p.shown());
  });
  const b = D.useCallback(() => {
    r(p.postponed()), $.setDeprecatedEnableSharingDialogWasShown(), m();
  }, [m, r]), h = G(_);
  return e(k, { animationProps: i, onPostpone: b, title: h ? "Enable Shared Groups" : "Enable Access to Shared Groups", isCloseButtonDisabled: x, children: e(K, { onSuccess: () => {
    r(p.succeeded()), m();
  }, onSubmit: () => {
    g(true), r(p.submitted());
  }, onFail: () => {
    r(p.caughtError()), g(false);
  }, isOwner: !!h }) });
}, ne = W(z(X));
export {
  ne as default
};
