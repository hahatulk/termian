/*
 * Recovered Termius UI module: SettingsSection-bc44d6bf.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bt as t, bu as n, bi as e, bP as d } from "./reconnectSaga-3b11f62e.js";
const m = ({ children: a }) => t(n, { variant: "subtitle1", color: "main", children: a }), r = ({ children: a, className: s }) => {
  const { classes: i } = o();
  return t("div", { className: d(s, i.settingsSection), children: a });
}, o = e()({ settingsSection: { padding: "20px", background: "var(--card-a)", borderRadius: "10px", width: "100%", maxWidth: "700px", margin: "30px auto 0", "&:last-child": { marginBottom: "60px" }, "@media only screen and (max-width: 1280px)": { maxWidth: "600px" }, "@media only screen and (max-width: 900px)": { maxWidth: "500px" }, "@media only screen and (max-width: 800px)": { width: "auto", margin: "30px 20px 0 20px" } } });
export {
  r as S,
  m as a
};
