/*
 * Recovered Termius UI module: triggerChoosePlanDialog-b93628a0.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { f1 as t, f2 as e, aH as g } from "./reconnectSaga-3b11f62e.js";
const r = async ({ funnelIdValue: o, sagas: a, proPurchaseOnly: s = false, isTriggeredByUser: l = false }) => {
  t(a, { dialog: "ChoosePlanDialog", target: e(), props: { funnelIdValue: o, proPurchaseOnly: s, isTriggeredByUser: l } }).catch(g);
};
export {
  r as t
};
