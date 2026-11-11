/*
 * Recovered Termius UI module: transitionScreenWrapper-62515248.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bt as e, zq as o } from "./reconnectSaga-3b11f62e.js";
function a(r) {
  return function(n) {
    return e(o, { onOk: n.onDialogClosed, onClose: n.onDialogClosed, isOpen: n.dialogState !== "closing", children: e(r, { ...n }) });
  };
}
export {
  a as t
};
