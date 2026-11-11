/*
 * Recovered Termius UI module: RadioOptionsList-6f3efe10.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { hI as B, hJ as q, hK as C, hL as w, hM as d, r as m, hN as U, hO as I, ze as J, zf as K, zg as A, bt as s, hQ as M, br as v, bs as Q, hR as _, zh as X, zi as oo, zj as eo, xU as ro, zk as ao, zl as to, xW as no, zm as so, bi as lo, cE as E, cF as O, zn as co, cX as T, bu as j } from "./reconnectSaga-3b11f62e.js";
function io(o) {
  return q("MuiFormControlLabel", o);
}
const uo = B("MuiFormControlLabel", ["root", "labelPlacementStart", "labelPlacementTop", "labelPlacementBottom", "disabled", "label", "error", "required", "asterisk"]), P = uo, po = ["checked", "className", "componentsProps", "control", "disabled", "disableTypography", "inputRef", "label", "labelPlacement", "name", "onChange", "required", "slotProps", "value"], mo = (o) => {
  const { classes: e, disabled: r, labelPlacement: a, error: t, required: n } = o, u = { root: ["root", r && "disabled", `labelPlacement${w(a)}`, t && "error", n && "required"], label: ["label", r && "disabled"], asterisk: ["asterisk", t && "error"] };
  return _(u, io, e);
}, fo = C("label", { name: "MuiFormControlLabel", slot: "Root", overridesResolver: (o, e) => {
  const { ownerState: r } = o;
  return [{ [`& .${P.label}`]: e.label }, e.root, e[`labelPlacement${w(r.labelPlacement)}`]];
} })(({ theme: o, ownerState: e }) => d({ display: "inline-flex", alignItems: "center", cursor: "pointer", verticalAlign: "middle", WebkitTapHighlightColor: "transparent", marginLeft: -11, marginRight: 16, [`&.${P.disabled}`]: { cursor: "default" } }, e.labelPlacement === "start" && { flexDirection: "row-reverse", marginLeft: 16, marginRight: -11 }, e.labelPlacement === "top" && { flexDirection: "column-reverse", marginLeft: 16 }, e.labelPlacement === "bottom" && { flexDirection: "column", marginLeft: 16 }, { [`& .${P.label}`]: { [`&.${P.disabled}`]: { color: (o.vars || o).palette.text.disabled } } })), ho = C("span", { name: "MuiFormControlLabel", slot: "Asterisk", overridesResolver: (o, e) => e.asterisk })(({ theme: o }) => ({ [`&.${P.error}`]: { color: (o.vars || o).palette.error.main } })), bo = m.forwardRef(function(e, r) {
  var a, t;
  const n = U({ props: e, name: "MuiFormControlLabel" }), { className: u, componentsProps: c = {}, control: l, disabled: p, disableTypography: f, label: R, labelPlacement: h = "end", required: k, slotProps: y = {} } = n, x = I(n, po), i = J(), b = (a = p ?? l.props.disabled) != null ? a : i == null ? void 0 : i.disabled, g = k ?? l.props.required, F = { disabled: b, required: g };
  ["checked", "name", "onChange", "value", "inputRef"].forEach((z) => {
    typeof l.props[z] > "u" && typeof n[z] < "u" && (F[z] = n[z]);
  });
  const $ = K({ props: n, muiFormControl: i, states: ["error"] }), N = d({}, n, { disabled: b, labelPlacement: h, required: g, error: $.error }), L = mo(N), G = (t = y.typography) != null ? t : c.typography;
  let S = R;
  return S != null && S.type !== A && !f && (S = s(A, d({ component: "span" }, G, { className: M(L.label, G == null ? void 0 : G.className), children: S }))), v(fo, d({ className: M(L.root, u), ownerState: N, ref: r }, x, { children: [m.cloneElement(l, F), g ? v(Q, { direction: "row", alignItems: "center", children: [S, v(ho, { ownerState: N, "aria-hidden": true, className: L.asterisk, children: [" ", "*"] })] }) : S] }));
}), go = bo, vo = X(s("path", { d: "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8z" }), "RadioButtonUnchecked"), Co = X(s("path", { d: "M8.465 8.465C9.37 7.56 10.62 7 12 7C14.76 7 17 9.24 17 12C17 13.38 16.44 14.63 15.535 15.535C14.63 16.44 13.38 17 12 17C9.24 17 7 14.76 7 12C7 10.62 7.56 9.37 8.465 8.465Z" }), "RadioButtonChecked"), Ro = C("span")({ position: "relative", display: "flex" }), ko = C(vo)({ transform: "scale(1)" }), yo = C(Co)(({ theme: o, ownerState: e }) => d({ left: 0, position: "absolute", transform: "scale(0)", transition: o.transitions.create("transform", { easing: o.transitions.easing.easeIn, duration: o.transitions.duration.shortest }) }, e.checked && { transform: "scale(1)", transition: o.transitions.create("transform", { easing: o.transitions.easing.easeOut, duration: o.transitions.duration.shortest }) }));
function Z(o) {
  const { checked: e = false, classes: r = {}, fontSize: a } = o, t = d({}, o, { checked: e });
  return v(Ro, { className: r.root, ownerState: t, children: [s(ko, { fontSize: a, className: r.background, ownerState: t }), s(yo, { fontSize: a, className: r.dot, ownerState: t })] });
}
const xo = m.createContext(void 0), Y = xo;
function So() {
  return m.useContext(Y);
}
function $o(o) {
  return q("MuiRadio", o);
}
const Po = B("MuiRadio", ["root", "checked", "disabled", "colorPrimary", "colorSecondary", "sizeSmall"]), V = Po, wo = ["checked", "checkedIcon", "color", "icon", "name", "onChange", "size", "className"], Fo = (o) => {
  const { classes: e, color: r, size: a } = o, t = { root: ["root", `color${w(r)}`, a !== "medium" && `size${w(a)}`] };
  return d({}, e, _(t, $o, e));
}, zo = C(oo, { shouldForwardProp: (o) => eo(o) || o === "classes", name: "MuiRadio", slot: "Root", overridesResolver: (o, e) => {
  const { ownerState: r } = o;
  return [e.root, e[`color${w(r.color)}`]];
} })(({ theme: o, ownerState: e }) => d({ color: (o.vars || o).palette.text.secondary }, !e.disableRipple && { "&:hover": { backgroundColor: o.vars ? `rgba(${e.color === "default" ? o.vars.palette.action.activeChannel : o.vars.palette[e.color].mainChannel} / ${o.vars.palette.action.hoverOpacity})` : ro(e.color === "default" ? o.palette.action.active : o.palette[e.color].main, o.palette.action.hoverOpacity), "@media (hover: none)": { backgroundColor: "transparent" } } }, e.color !== "default" && { [`&.${V.checked}`]: { color: (o.vars || o).palette[e.color].main } }, { [`&.${V.disabled}`]: { color: (o.vars || o).palette.action.disabled } }));
function Mo(o, e) {
  return typeof e == "object" && e !== null ? o === e : String(o) === String(e);
}
const W = s(Z, { checked: true }), H = s(Z, {}), Io = m.forwardRef(function(e, r) {
  var a, t;
  const n = U({ props: e, name: "MuiRadio" }), { checked: u, checkedIcon: c = W, color: l = "primary", icon: p = H, name: f, onChange: R, size: h = "medium", className: k } = n, y = I(n, wo), x = d({}, n, { color: l, size: h }), i = Fo(x), b = So();
  let g = u;
  const F = ao(R, b && b.onChange);
  let $ = f;
  return b && (typeof g > "u" && (g = Mo(b.value, n.value)), typeof $ > "u" && ($ = b.name)), s(zo, d({ type: "radio", icon: m.cloneElement(p, { fontSize: (a = H.props.fontSize) != null ? a : h }), checkedIcon: m.cloneElement(c, { fontSize: (t = W.props.fontSize) != null ? t : h }), ownerState: x, classes: i, name: $, checked: g, onChange: F, ref: r, className: M(i.root, k) }, y));
}), No = Io;
function Lo(o) {
  return q("MuiFormGroup", o);
}
B("MuiFormGroup", ["root", "row", "error"]);
const Go = ["className", "row"], Bo = (o) => {
  const { classes: e, row: r, error: a } = o;
  return _({ root: ["root", r && "row", a && "error"] }, Lo, e);
}, qo = C("div", { name: "MuiFormGroup", slot: "Root", overridesResolver: (o, e) => {
  const { ownerState: r } = o;
  return [e.root, r.row && e.row];
} })(({ ownerState: o }) => d({ display: "flex", flexDirection: "column", flexWrap: "wrap" }, o.row && { flexDirection: "row" })), Uo = m.forwardRef(function(e, r) {
  const a = U({ props: e, name: "MuiFormGroup" }), { className: t, row: n = false } = a, u = I(a, Go), c = J(), l = K({ props: a, muiFormControl: c, states: ["error"] }), p = d({}, a, { row: n, error: l.error }), f = Bo(p);
  return s(qo, d({ className: M(f.root, t), ownerState: p, ref: r }, u));
}), _o = Uo, Do = ["actions", "children", "defaultValue", "name", "onChange", "value"], Ao = m.forwardRef(function(e, r) {
  const { actions: a, children: t, defaultValue: n, name: u, onChange: c, value: l } = e, p = I(e, Do), f = m.useRef(null), [R, h] = to({ controlled: l, default: n, name: "RadioGroup" });
  m.useImperativeHandle(a, () => ({ focus: () => {
    let i = f.current.querySelector("input:not(:disabled):checked");
    i || (i = f.current.querySelector("input:not(:disabled)")), i && i.focus();
  } }), []);
  const k = no(r, f), y = so(u), x = m.useMemo(() => ({ name: y, onChange(i) {
    h(i.target.value), c && c(i, i.target.value);
  }, value: R }), [y, c, h, R]);
  return s(Y.Provider, { value: x, children: s(_o, d({ role: "radiogroup", ref: k }, p, { children: t })) });
}), Eo = Ao;
function To({ label: o, value: e, children: r, className: a, onChange: t, ...n }) {
  const { classes: u, cx: c } = D();
  return v(Eo, { value: e, className: c(u.optionsGroup, a), ...n, onChange: (l) => {
    t == null || t(l, l.target.value);
  }, children: [o && s(co, { children: o }), r] });
}
function jo(o) {
  const { value: e, isChecked: r, label: a, children: t, startAdornment: n, className: u, classes: c } = o, { classes: l, cx: p } = D();
  return s(go, { className: p(l.option, u), label: v(Q, { direction: "row", gap: "10px", children: [n && s(T, { className: p(l.optionStartAdornment, c == null ? void 0 : c.startAdornment), children: n }), v(T, { children: [s(j, { variant: "body1", color: "main", bold: true, children: a }), t && s(j, { variant: "body2", children: t })] })] }), value: e, control: s(No, { checked: r, color: "primary" }) });
}
function Vo() {
  const { classes: o } = D();
  return s("div", { className: o.optionsSeparator });
}
const D = lo()({ optionsGroup: { display: "flex", flexDirection: "column", gap: 10 }, optionsSeparator: { width: "auto", height: "1px", background: "var(--light-grey-transparent-10)", margin: "10px 0", ".termius-dark-theme &": { background: "var(--dark-grey-transparent-10)" } }, optionStartAdornment: { display: "flex", padding: "10px", borderRadius: "25%", height: "fit-content", backgroundColor: "var(--card-b)" }, option: { display: "flex", gap: "10px", flexDirection: "row-reverse", justifyContent: "space-between", width: "100%", borderRadius: "15px", padding: "14px", border: "1px solid var(--border-color)", margin: 0, transition: `border-color ${E.shorter}ms ${O.sharp}, background-color ${E.short}ms ${O.sharp}`, "--border-color": "var(--light-grey-4)", "--radio-color": "var(--light-grey-3)", "--background-accent": "var(--white)", ".termius-dark-theme &": { "--border-color": "var(--dark-grey-5)", "--radio-color": "var(--dark-grey-7)", "--background-accent": "var(--dark-grey-3)" }, "&:hover": { "--border-color": "var(--light-grey-3)", ".termius-dark-theme &": { "--border-color": "var(--dark-grey-6)" } }, "& .MuiSvgIcon-root": { width: "16px", height: "16px" }, "& .MuiRadio-root": { padding: 0, color: "var(--radio-color)" }, "& .MuiRadio-root.Mui-checked": { color: "var(--accent) !important" }, "&:has(.MuiRadio-root.Mui-checked)": { borderColor: "var(--accent)", backgroundColor: "var(--background-accent)" } } });
export {
  To as R,
  jo as a,
  Vo as b
};
