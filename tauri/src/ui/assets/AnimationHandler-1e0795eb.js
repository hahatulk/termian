/*
 * Recovered Termius UI module: AnimationHandler-1e0795eb.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as g, vA as h, bP as s, bO as P } from "./reconnectSaga-3b11f62e.js";
const T = (i) => {
  const { children: t, isContainer: e, order: a, animationProps: r } = i, { classes: n } = L();
  if (!r) return t;
  const { mounted: l, opacity: f, translate: u, opacityContainer: m, onTransitionEndContainer: p, onTransitionEndContent: d, contentRef: y, containerRef: C } = r, E = s({ [n.container]: true, [n.transitionContainer]: true, [n[m ? "opacityOn" : "opacityOff"]]: true }), O = s({ [n[l ? "transition" : `transitionElement${a > 1 ? a : ""}`]]: true, [n[f ? "opacityOn" : "opacityOff"]]: true, [n[u ? "translateYOff" : "translateYOn"]]: true }), o = i.isLastElement || a === 3, R = e ? E : O, c = () => e ? C : o ? y : null, A = () => {
    if (e) return p;
    if (o) return d;
  }, b = () => typeof t.type == "function" ? { innerRef: c() } : { ref: c() };
  return P.cloneElement(t, { className: s(R, t.props.className), onTransitionEnd: A(), ...b() });
};
T.defaultProps = { isContainer: false, isLastElement: false, order: 1 };
const L = g()({ container: { height: "100%", width: "100%" }, ...h });
export {
  T as A
};
