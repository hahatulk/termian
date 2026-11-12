/*
 * Recovered Termius UI module: UserListItem-e815dc7d.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as I, br as n, bs as s, bt as i, b$ as L, c0 as N, c1 as C, bu as d } from "./reconnectSaga-3b11f62e.js";
const w = ({ component: m = "li", user: a, avatar: u, disabled: o = false, className: h, titleClassName: b, subTitleClassName: p, endAdornmentClassName: x, inlineIcons: g = null, endAdornment: c = null, iconSize: f = "extraLarge", ...v }) => {
  const { classes: e, cx: r } = S(), k = "avatar" in a && !!a.avatar, t = "full_name" in a ? a.full_name : "", l = "email" in a ? a.email : "", y = "user_id" in a ? a.user_id : 0;
  return n(s, { ...v, component: m, className: r(e.userListItem, { [e.disabled]: o }, h), children: [i(L, { avatar: k ? a.avatar : u, color: N(l, y), letter: C({ email: l, fullName: t }), size: f }), n(s, { minWidth: "0", children: [n(s, { flexDirection: "row", children: [i(d, { color: "main", className: r(e.email, { [e.disabled]: o }, b), ellipsis: true, children: t || l }), g] }), !!t && i(d, { color: "secondary", variant: "body2", className: r(e.email, p), ellipsis: true, children: l })] }), !!c && i(s, { className: r(e.endAdornment, x), children: c })] });
}, S = I()({ userListItem: { flexDirection: "row", alignItems: "center", gap: "10px", width: "100%", minHeight: "60px", backgroundColor: "inherit", borderRadius: "15px", padding: "10px", cursor: "default", transition: "all 0.1s ease", "&:hover": { backgroundColor: "var(--card-b)", ".termius-dark-theme &": { backgroundColor: "var(--card-c)" } } }, email: { maxWidth: "560px" }, endAdornment: { flexShrink: 0, marginLeft: "auto" }, disabled: { pointerEvents: "none", "&:hover": { backgroundColor: "inherit" } } });
export {
  w as U
};
