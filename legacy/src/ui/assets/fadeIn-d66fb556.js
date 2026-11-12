/*
 * Recovered Termius UI module: fadeIn-d66fb556.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { rC as t } from "./reconnectSaga-3b11f62e.js";
const p = (n) => {
  const { dialogState: o, onDialogOpened: i, onDialogClosed: e } = n;
  return t({ opacity: o === "opened" ? 1 : 0, from: { opacity: 0 }, to: { opacity: 1 }, onRest: () => {
    o === "opening" && i();
  }, onProps: () => {
    o === "closing" && e();
  }, config: { duration: 200 } });
};
export {
  p as u
};
