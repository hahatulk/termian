/*
 * Recovered Termius UI module: SuccessScreen-cc065469.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as b, bt as t, cv as a, br as o, bs as s, bu as r, x8 as k, bP as S } from "./reconnectSaga-3b11f62e.js";
const l = "" + new URL("CheckIconWithCloudsDark1x-43c530e9.webp", import.meta.url).href, C = "" + new URL("CheckIconWithCloudsDark2x-1ece010b.webp", import.meta.url).href, m = "" + new URL("CheckIconWithCloudsLight1x-1003ce42.webp", import.meta.url).href, f = "" + new URL("CheckIconWithCloudsLight2x-536c6ee5.webp", import.meta.url).href, y = (d) => {
  const { onClose: c, title: p, description: i, hasCenterButton: h = false, buttonClassName: x, mainSectionClassName: u } = d, { classes: e, cx: n } = w(), g = h ? t(a, { className: e.centerButton, onClick: c, color: "accent", children: "Done" }) : t("div", { className: n(e.buttonWrapper, x), children: t(a, { className: e.button, onClick: c, color: "accent", children: "Done" }) });
  return o(s, { direction: "column", alignItems: "center", width: "100%", height: "100%", children: [o(s, { direction: "column", alignItems: "center", width: "100%", height: "466px", className: u, children: [t(r, { variant: "h2", color: "main", align: "center", children: p }), i && t(r, { variant: "body1", color: "main", align: "center", className: e.description, children: i }), o("div", { children: [t(k, { className: S(e.confettiAnimation, { [e.confettiAnimationWithDescription]: !!i }) }), t("img", { className: n(e.image, e.darkModeImage), src: l, srcSet: `${l} 1x, ${C} 2x`, alt: "" }), t("img", { className: n(e.image, e.lightModeImage), src: m, srcSet: `${m} 1x, ${f} 2x`, alt: "" })] })] }), g] });
}, w = b()({ buttonWrapper: { position: "absolute", bottom: "40px", borderTop: "1px solid var(--card-b)", ".termius-dark-theme &": { borderColor: "var(--card-c)" }, width: "100%", display: "flex", flexDirection: "column", justifyContent: "center", alignItems: "end", height: "74px", "@media (max-height: 520px)": { bottom: "20px" } }, button: { margin: "20px", width: "67px" }, centerButton: { width: "300px", bottom: "40px" }, description: { paddingTop: "10px" }, image: { paddingTop: "100px" }, darkModeImage: { display: "none", ".termius-dark-theme &": { display: "block" } }, lightModeImage: { display: "block", ".termius-dark-theme &": { display: "none" } }, confettiAnimation: { position: "absolute", top: "100px", left: "calc((100% - 200px) / 2)", width: "200px", height: "200px", zIndex: 1 }, confettiAnimationWithDescription: { top: "130px" } });
export {
  y as S
};
