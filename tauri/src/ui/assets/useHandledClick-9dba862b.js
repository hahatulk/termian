/*
 * Recovered Termius UI module: useHandledClick-9dba862b.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { yd as i, r as s, yk as u } from "./reconnectSaga-3b11f62e.js";
const m = ({ onClick: a }) => {
  const r = i(), [c, e] = s.useState("default"), [t, o] = s.useState();
  return { handleClick: async () => {
    t && clearTimeout(t);
    try {
      e("loading"), await r(a()), e("success");
    } catch (n) {
      if (n instanceof u) return;
      e("error");
    }
    o(setTimeout(() => e("default"), 5e3));
  }, status: c };
};
export {
  m as u
};
