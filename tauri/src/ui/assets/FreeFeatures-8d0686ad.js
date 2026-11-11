/*
 * Recovered Termius UI module: FreeFeatures-8d0686ad.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { br as o, bO as r, bt as e, bi as b, fu as S, bs as p, bu as h, dT as c, cX as k, cv as m, r as f, dS as w } from "./reconnectSaga-3b11f62e.js";
import { A as y } from "./AuthorizationStep-05da7af3.js";
import { T as F, S as d } from "./SuccessStep-750a80f6.js";
import { D as x, a as A } from "./DialogPanel-fd3d3785.js";
import { u as T } from "./useWizard-d6695218.js";
import "./AuthorizationForm-253327c1.js";
import "./OTPToken-a5994a9f.js";
import "./InputDialog-b3f32c9f.js";
import "./AlertDialog-b50b9ed7.js";
import "./Button-90700c84.js";
import "./PasswordInput-9d57733c.js";
import "./OtpLogin-e15265bf.js";
const I = "" + new URL("promo-autocomplete-dark-88839b0d.webp", import.meta.url).href, U = "" + new URL("promo-autocomplete-light-b6a2fd0c.webp", import.meta.url).href, C = "" + new URL("promo-cloudVault-dark-802f4fbc.webp", import.meta.url).href, L = "" + new URL("promo-cloudVault-light-1ddad437.webp", import.meta.url).href, v = "" + new URL("promo-snippets-dark-9581d032.webp", import.meta.url).href, P = "" + new URL("promo-snippets-light-af301ba0.webp", import.meta.url).href, z = "" + new URL("promo-terminal-sharing-dark-1a04defe.webp", import.meta.url).href, R = "" + new URL("promo-terminal-sharing-light-32f218d0.webp", import.meta.url).href, s = { aiForSnippets: { title: o(r.Fragment, { children: ["Create ", e("mark", { children: "Snippets with AI" })] }), description: e(r.Fragment, { children: "Describe an action and generate a Snippet, bypassing window switching and online searches." }), highlight: e(r.Fragment, { children: "1000 prompts per month free" }) }, aiForAutocomplete: { title: o(r.Fragment, { children: ["Use ", e("mark", { children: "AI in terminal" })] }), description: e(r.Fragment, { children: "Type your request in the terminal and generate a command, bypassing window switching and online searches." }), highlight: e(r.Fragment, { children: "1000 prompts per month free" }) }, terminalSharing: { title: o(r.Fragment, { children: ["Terminal ", e("mark", { children: "Multiplayer" })] }), description: e(r.Fragment, { children: "Send a link to your teammates and troubleshoot issues together in the same terminal session" }), highlight: e(r.Fragment, { children: "100 sessions per month for free" }) }, backUpAndSync: { title: o(r.Fragment, { children: ["Set up your ", e("br", {}), e("mark", { children: "cloud vault" })] }), description: e(r.Fragment, { children: "Backup, sync all you data in an encrypted cloud vault and access it from your mobile and desktop" }), highlight: e(r.Fragment, { children: "Trusted by 30,000 professionals " }) } }, g = { aiForSnippets: { left: "Later", right: "Enable AI" }, aiForAutocomplete: { left: "Later", right: "Enable AI" }, terminalSharing: { left: "Cancel", right: "Enable Multiplayer" }, backUpAndSync: { left: "Later", right: "Set up" } };
function B(n) {
  const { variant: t, onClose: l, onContinue: a } = n, { classes: i } = D();
  return e(x, { size: { width: 720, height: 540 }, content: e(S, { in: true, timeout: { enter: 300, exit: 300 }, children: o(p, { className: i.content, children: [o(p, { direction: "column", width: "310px", gap: "20px", children: [e(h, { variant: "h1", highlighter: true, children: s[t].title }), e(h, { variant: "body1", color: "secondary", className: i.description, children: s[t].description }), e(c, { when: !!s[t].highlight, children: e(h, { variant: "body1", color: "secondary", children: e("span", { className: i.highlight, children: s[t].highlight }) }) })] }), e(k, { width: 310, height: "100%", children: e(N, { variant: t }) })] }) }), actions: e(A, { start: e(m, { type: "button", variant: "text", onClick: l, children: g[t].left }), end: e(m, { type: "button", color: "accent", onClick: a, children: g[t].right }) }) });
}
const D = b()({ content: { alignItems: "center", flexDirection: "row", gap: "20px", height: "100%" }, highlight: { position: "relative", padding: "5px 10px", WebkitTextFillColor: "transparent", backgroundImage: "var(--gradient-universal-main)", WebkitBackgroundClip: "text", backgroundClip: "text", zIndex: 0, "&::before": { content: '""', background: "var(--main-gradient-fill)", position: "absolute", left: 0, right: 0, top: 0, bottom: 0, zIndex: -1, borderRadius: "7px" }, ".termius-dark-theme &": { color: "white", WebkitTextFillColor: "inherit", backgroundImage: "none", "&::before": { background: "var(--main-gradient-outline)" } } }, description: { minHeight: "84px" } }), u = { aiForSnippets: { light: P, dark: v }, aiForAutocomplete: { light: U, dark: I }, terminalSharing: { light: R, dark: z }, backUpAndSync: { light: L, dark: C } };
function N(n) {
  const { classes: t } = W();
  return e(k, { className: t.root, children: e(F, { light: { className: t.image, srcSet: `${u[n.variant].light} 2x` }, dark: { className: t.image, srcSet: `${u[n.variant].dark} 2x` } }) });
}
const W = b()({ root: { height: "100%", textAlign: "center" }, image: { zIndex: 1, height: "100%", width: "auto" } }), V = { aiForSnippets: "Snippets With AI", aiForAutocomplete: "Text To Command With AI", terminalSharing: "Terminal Sharing Screen", backUpAndSync: "Backup & Sync" }, E = { aiForSnippets: "aiPower", aiForAutocomplete: "aiPower", terminalSharing: "terminalSharing", backUpAndSync: "backUpAndSync" }, Z = f.forwardRef(({ closeDialog: n, variant: t }, l) => {
  const [a, i] = T(["promotion", "authorization", "success-sign-up", "success-sign-in"]);
  return e(S, { in: true, children: e("div", { ref: l, children: o(w, { children: [e(c, { when: a === "promotion", children: e(B, { variant: t, onClose: () => n({ authorized: false }), onContinue: i.forward }) }), e(c, { when: a === "authorization", children: e(y, { variant: E[t], onBack: i.backward, onSignUp: () => i.to("success-sign-up"), onSignIn: () => i.to("success-sign-in"), createAccountScreenType: V[t] }) }), e(c, { when: a === "success-sign-up", children: e(d, { type: "sign-up", variant: t, onComplete: () => n({ authorized: true }) }) }), e(c, { when: a === "success-sign-in", children: e(d, { type: "sign-in", variant: t, onComplete: () => n({ authorized: true }) }) })] }) }) });
});
export {
  Z as default
};
