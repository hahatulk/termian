/*
 * Recovered Termius UI module: CloseAndBackDialogSection-7ee44c0d.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as n, br as l, bP as k, bt as o, en as e, yI as h, vI as m } from "./reconnectSaga-3b11f62e.js";
const g = (s) => {
  const { onClose: c, onBack: i, className: r, useBackButton: t } = s, { classes: a } = v();
  return l("div", { className: k(r, a.root), children: [t && o(e, { onClick: i, className: a.backIcon, children: o(h, { width: 16, height: 12 }) }), o(e, { onClick: () => c(), className: a.closeDialog, children: o(m, { width: 12, height: 12 }) })] });
}, v = n()({ root: { "--back-icon": "var(--light-grey-3)", "--close-icon": "var(--light-grey-1)", ".termius-dark-theme &": { "--back-icon": "var(--dark-grey-7)", "--close-icon": "var(--dark-grey-7)" }, position: "relative" }, closeDialog: { position: "absolute", top: "10px", right: "10px", color: "var(--close-icon)", transition: "all .3s", "&:hover": { "--close-icon": "var(--dark-grey-1)", ".termius-dark-theme &": { "--close-icon": "var(--white)" } } }, backIcon: { position: "absolute", top: "10px", left: "10px", color: "var(--back-icon)", transition: "all .3s", "&:hover": { "--back-icon": "var(--dark-grey-1)", ".termius-dark-theme &": { "--back-icon": "var(--white)" } } } });
export {
  g as C
};
