/*
 * Recovered Termius UI module: PurchaseFailedDialog-a521ab97.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bq as u, r as e, yR as l, yS as i, bt as h, yT as y } from "./reconnectSaga-3b11f62e.js";
const b = ({ message: n, purchaseLocation: c, upgradeToProFunnelID: r, closeDialog: t }) => {
  const a = u(), s = e.useCallback(() => {
    a(l()), t();
  }, [a, t]), o = e.useCallback(() => {
    a(i({ buyNowButtonLocation: c, upgradeToProFunnelID: r })), t();
  }, [a, c, r, t]);
  return h(y, { title: "Purchase failed", message: n, onClose: s, actionButtons: [{ children: "Cancel", onClick: s }, { children: "Retry", onClick: o, color: "accent", autoFocus: true }] });
};
export {
  b as default
};
