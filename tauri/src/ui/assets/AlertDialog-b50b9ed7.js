/*
 * Recovered Termius UI module: AlertDialog-b50b9ed7.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { cu as s, dZ as u, dF as m, bt as y, cs as D } from "./reconnectSaga-3b11f62e.js";
const i = ({ title: e, text: o, children: p, onOk: c, onClose: t, dark: n, type: g = "primary", blockKeyboardInput: a = true }) => (m("keydown", (r) => {
  a && r.stopPropagation(), r.code === "Escape" && (t == null || t());
}, { capture: a }), y(D, { title: e, titleColor: g, onClose: t, onSubmit: c, dark: n, text: o, children: p })), l = s(i, { hasBackdrop: true }), w = u.promisify(s(i));
export {
  i as A,
  l as a,
  w as r
};
