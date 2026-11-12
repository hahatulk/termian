/*
 * Recovered Termius UI module: index-382b939e.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { aK as v, bi as y, br as u, bt as e, cy as m, bu as f, yl as C, cN as B, yd as N, bj as k, gG as O, cO as E, r as a, ew as j, yk as R, dE as z } from "./reconnectSaga-3b11f62e.js";
import { S as L } from "./SettingsTabContent-ce30872f.js";
const $ = v("GET_PRODUCTBOARD_TOKEN"), D = "" + new URL("background_circle-1452d38b.svg", import.meta.url).href, M = () => {
  const { classes: s } = _();
  return u("div", { className: s.offline, children: [e(m, { top: 40, children: e(f, { variant: "h2", align: "center", children: "Offline" }) }), e(m, { top: 40, children: e(f, { fontSize: 14, children: "Please make sure that your internet connection is active to proceed." }) }), e("div", { className: s.background }), e("div", { className: s.imageContainer, children: e(C, { className: s.offlineIcon }) })] });
}, _ = y()({ offline: { background: "inherit", color: "var(--text-primary)", position: "absolute", height: "100%", width: "100%", display: "flex", flexDirection: "column", alignItems: "center", zIndex: 1 }, offlineIcon: { width: "200px", height: "200px", "@media screen and (max-height: 570px)": { width: "120px", height: "120px" } }, imageContainer: { position: "absolute", height: "100%", width: "100%", display: "flex", alignItems: "center", justifyContent: "center" }, background: { pointerEvents: "none", position: "absolute", left: "50%", top: "50%", backgroundImage: `url(${D})`, backgroundRepeat: "no-repeat", backgroundPosition: "top", backgroundSize: "130%", transform: "translate(-50%, 0)", width: "100%", height: "50%", zIndex: -1 } }), x = "https://ideas.termius.com/", q = "https://ideas.teams.termius.com";
function G(s, n) {
  switch (s) {
    case "main":
      return n ? q : x;
    case "keywords highlighting":
      return "https://ideas.termius.com/c/70-highlight-terminal-syntax-by-a-regex";
    default:
      return x;
  }
}
const K = (s) => {
  const { page: n } = s, h = B(), { classes: r } = U(), g = N(), o = k(O), p = k(E), [i, c] = a.useState(false), [b, S] = a.useState(false), [l, P] = a.useState(null), w = a.useCallback(async () => {
    try {
      const t = await g(j(h, $));
      P(t), c(true);
    } catch (t) {
      if (t instanceof R) return;
      if ((t == null ? void 0 : t.status) === 0) {
        S(true);
        return;
      }
      c(true);
    }
  }, [g, h]);
  a.useEffect(() => {
    o ? w() : c(true);
  }, []);
  const d = a.useMemo(() => ({ className: r.iframe, frameBorder: "0", sandbox: "allow-same-origin allow-scripts" }), [r.iframe]), I = a.useMemo(() => {
    const t = G(n || "main", p);
    if (b) return e(M, {});
    if (!o && i) return e("iframe", { title: "Productboard", src: t, ...d });
    if (l && i) {
      const T = `${t}?token=${l.jwt}`;
      return e("iframe", { title: "productboard", src: T, ...d });
    }
    return null;
  }, [d, i, b, l, o, p, n]);
  return e(L, { children: u("div", { className: r.container, children: [u(f, { className: r.loaderText, children: [e(m, { right: 10, top: 2, children: e(z, { className: r.progress, size: 20 }) }), "Loading ideas portal..."] }), I] }) });
}, U = y()({ container: { background: "inherit", display: "flex", alignItems: "center", justifyContent: "center", position: "absolute", top: "0", left: "0", right: "0", bottom: "0" }, progress: { color: "var(--accent)" }, loaderText: { marginBottom: "80px", display: "flex", alignItems: "center", color: "var(--text-primary)" }, iframe: { position: "absolute", height: "100%", width: "100%" } }), W = K;
export {
  W as default
};
