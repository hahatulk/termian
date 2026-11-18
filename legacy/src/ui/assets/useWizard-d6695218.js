/*
 * Recovered Termius UI module: useWizard-d6695218.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as s } from "./reconnectSaga-3b11f62e.js";
function I(e) {
  const [o, r] = s.useState(0), c = e.length - 1;
  return [e[o], { forward: () => {
    r((t) => {
      const n = t + 1;
      return Math.min(n, c);
    });
  }, backward: () => {
    r((t) => {
      const n = t - 1;
      return Math.max(n, 0);
    });
  }, to: (t) => {
    r((n) => {
      const a = e.indexOf(t);
      return a === -1 ? n : a;
    });
  } }];
}
export {
  I as u
};
