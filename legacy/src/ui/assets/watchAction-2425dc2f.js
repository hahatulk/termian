/*
 * Recovered Termius UI module: watchAction-2425dc2f.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { j1 as a } from "./reconnectSaga-3b11f62e.js";
function* e(t, c) {
  yield* a(t, c);
}
const s = (t, c, n) => {
  const r = t.run(e, c, n);
  return () => r.cancel();
};
export {
  s as w
};
