/*
 * Recovered Termius UI module: OTPToken-a5994a9f.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as h, ft as f, r as g, br as i, bt as r, bu as k, cy as y, fv as S, jV as v, k1 as x, cv as C } from "./reconnectSaga-3b11f62e.js";
const F = (n) => {
  var s;
  const { submitCallback: t, description: a, phoneWrapper: c = void 0 } = n, { classes: l } = T(), { register: u, handleSubmit: d, formState: e, setError: o } = f({ mode: "onSubmit", reValidateMode: "onChange" }), p = g.useCallback(async ({ otp_token: m }) => {
    try {
      await t(m.replaceAll(" ", ""));
    } catch (b) {
      o("otp_token", { message: b.message || "Unknown error" });
    }
  }, [o, t]);
  return i("form", { onSubmit: d(p), children: [r(k, { color: "secondary", className: l.description, children: a || "Enter the verification code from your 2FA provider." }), c, i(y, { top: 20, bottom: 20, children: [r(S, { variant: "outlined", label: "Verification code", required: true, fullWidth: true, autoFocus: true, InputProps: u("otp_token", { required: "The Verification code is required" }), error: !!e.errors.otp_token }), r(v, { in: !!e.errors.otp_token, children: r(x, { status: "error", children: (s = e.errors.otp_token) == null ? void 0 : s.message }) })] }), r(C, { disabled: e.isSubmitting || !e.isDirty, progress: e.isSubmitting, size: "medium", color: "accent", fullWidth: true, type: "submit", children: "Continue" })] });
}, T = h()({ description: { textAlign: "center" } });
export {
  F as O
};
