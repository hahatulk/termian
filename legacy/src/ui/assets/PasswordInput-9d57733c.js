/*
 * Recovered Termius UI module: PasswordInput-9d57733c.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bt as t, p8 as r, lr as a, Ah as l, r as i, ms as n } from "./reconnectSaga-3b11f62e.js";
function d({ visible: o, onToggle: s }) {
  return t(r, { "aria-label": "Toggle password visibility", onClick: s, tabIndex: -1, children: o ? t(a, { style: { color: "var(--light-grey-3)" } }) : t(l, { style: { color: "var(--light-grey-3)" } }) });
}
const u = (o) => {
  const [s, e] = i.useState(false);
  return t(n, { ...o, type: s ? "text" : "password", endAdornment: t(d, { visible: s, onToggle: () => e(!s) }) });
};
export {
  u as P
};
