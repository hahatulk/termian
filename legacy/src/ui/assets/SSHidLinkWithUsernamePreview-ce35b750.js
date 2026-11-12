/*
 * Recovered Termius UI module: SSHidLinkWithUsernamePreview-ce35b750.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as n, bt as e, bu as r, br as l } from "./reconnectSaga-3b11f62e.js";
function d(a) {
  const { cx: h, classes: i } = t(), { username: s = "" } = a;
  return s.length > 0 ? e(r, { ellipsis: true, highlighter: "sshid-gradient", className: i.preview, children: l("mark", { children: ["$ curl https://sshid.io/", s] }) }) : l(r, { children: [e(r, { className: i.preview, display: "inline", highlighter: "sshid-gradient", children: e("mark", { children: "$ curl https://sshid.io/" }) }), e(r, { display: "inline", highlighter: "sshid-gradient", className: h(i.preview, i.placeholder), children: e("mark", { children: "username" }) })] });
}
const t = n()({ preview: { fontFamily: "Source Code Pro", color: "var(--sshid-color-purple)" }, placeholder: { opacity: 0.5 } });
export {
  d as S
};
