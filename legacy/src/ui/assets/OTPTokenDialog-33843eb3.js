/*
 * Recovered Termius UI module: OTPTokenDialog-33843eb3.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { dZ as s, cu as n, r as c, br as l, bO as m, bt as r, e1 as d, cy as p } from "./reconnectSaga-3b11f62e.js";
import { O as f } from "./OTPToken-a5994a9f.js";
function b({ onOk: a, onClose: e, submitCallback: t }) {
  const i = c.useCallback(async (o) => {
    await t(o), a(o);
  }, [a, t]);
  return l(m.Fragment, { children: [r(d, { color: "primary", endAdornment: "close", title: "Enter verification code", onEndAdornmentClick: e }), r(p, { top: 25, right: 20, bottom: 25, left: 20, children: r(f, { submitCallback: i }) })] });
}
const y = s.promisify(n(b));
export {
  y as r
};
