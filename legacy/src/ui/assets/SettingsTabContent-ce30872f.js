/*
 * Recovered Termius UI module: SettingsTabContent-ce30872f.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as n, bt as o, bP as r } from "./reconnectSaga-3b11f62e.js";
function l(t) {
  const { className: e, children: s } = t, { classes: a } = c();
  return o("div", { className: r(e, a.settingsTabContent), children: s });
}
const c = n()({ settingsTabContent: { "--tab-content-color": "var(--light-grey-6)", height: "100%", width: "100%", overflow: "auto", background: "var(--tab-content-color)", ".termius-dark-theme &": { "--tab-content-color": "var(--dark-grey-2)" } } });
export {
  l as S
};
