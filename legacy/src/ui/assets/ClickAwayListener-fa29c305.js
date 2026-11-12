/*
 * Recovered Termius UI module: ClickAwayListener-fa29c305.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as n, xW as x, xY as C, xZ as h, bt as T } from "./reconnectSaga-3b11f62e.js";
function v(s) {
  return s.substring(2).toLowerCase();
}
function b(s, o) {
  return o.documentElement.clientWidth < s.clientX || o.documentElement.clientHeight < s.clientY;
}
function w(s) {
  const { children: o, disableReactTree: R = false, mouseEvent: u = "onClick", onClickAway: L, touchEvent: i = "onTouchEnd" } = s, f = n.useRef(false), c = n.useRef(null), l = n.useRef(false), d = n.useRef(false);
  n.useEffect(() => (setTimeout(() => {
    l.current = true;
  }, 0), () => {
    l.current = false;
  }), []);
  const k = x(o.ref, c), a = C((e) => {
    const t = d.current;
    d.current = false;
    const r = h(c.current);
    if (!l.current || !c.current || "clientX" in e && b(e, r)) return;
    if (f.current) {
      f.current = false;
      return;
    }
    let E;
    e.composedPath ? E = e.composedPath().indexOf(c.current) > -1 : E = !r.documentElement.contains(e.target) || c.current.contains(e.target), !E && (R || !t) && L(e);
  }), p = (e) => (t) => {
    d.current = true;
    const r = o.props[e];
    r && r(t);
  }, m = { ref: k };
  return i !== false && (m[i] = p(i)), n.useEffect(() => {
    if (i !== false) {
      const e = v(i), t = h(c.current), r = () => {
        f.current = true;
      };
      return t.addEventListener(e, a), t.addEventListener("touchmove", r), () => {
        t.removeEventListener(e, a), t.removeEventListener("touchmove", r);
      };
    }
  }, [a, i]), u !== false && (m[u] = p(u)), n.useEffect(() => {
    if (u !== false) {
      const e = v(u), t = h(c.current);
      return t.addEventListener(e, a), () => {
        t.removeEventListener(e, a);
      };
    }
  }, [a, u]), T(n.Fragment, { children: n.cloneElement(o, m) });
}
export {
  w as C
};
