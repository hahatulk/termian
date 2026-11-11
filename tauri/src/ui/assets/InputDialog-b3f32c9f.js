/*
 * Recovered Termius UI module: InputDialog-b3f32c9f.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as S, dZ as B, cu as D, r as T, bt as r, cs as I, e3 as k, fv as C } from "./reconnectSaga-3b11f62e.js";
const q = (a) => a.length === 0 ? "Required" : "";
function w(a) {
  const { onOk: u, onClose: i, type: c = "text", title: d = "", text: p, value: s, validator: e = q, titleButton: f, dark: m } = a, [t, x] = T.useState({ value: s || "", errorText: "" }), { classes: g } = F(), v = e ? !!e(t.value) : false, h = [{ children: f || "Confirm", color: "accent", disabled: v, type: "submit" }], b = (o) => {
    o.preventDefault(), u(t.value);
  }, l = (o) => {
    const { value: n } = o.target, y = e ? e(n) : "";
    x({ ...t, value: n, errorText: y });
  };
  return r(I, { title: d, onClose: i, onSubmit: b, actionButtons: h, dark: m, children: r(k, { children: r(C, { label: p, error: !!t.errorText, helperText: t.errorText || " ", type: c, defaultValue: s, onChange: l, onBlur: l, fullWidth: true, autoFocus: true, className: g.input }) }) });
}
const F = S()({ input: { marginBottom: "-10px" } }), V = B.promisify(D(w));
export {
  w as I,
  V as r
};
