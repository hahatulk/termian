/*
 * Recovered Termius UI module: SettingsSwitch-d35e9651.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { br as s, cy as e, bt as t, ct as n, bu as o } from "./reconnectSaga-3b11f62e.js";
const h = ({ topMargin: l = 20, bottomMargin: c, fontSize: i, description: a, ...r }) => s(e, { top: l, bottom: c, children: [t(n, { ...r, fullWidth: true, label: t(o, { color: "primary", fontSize: i ?? 12, children: r.label }) }), a && t(o, { variant: "body2", color: "secondary", children: a })] });
export {
  h as S
};
