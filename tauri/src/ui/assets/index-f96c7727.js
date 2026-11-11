/*
 * Recovered Termius UI module: index-f96c7727.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bi as b, bt as e, cH as w, br as p, bP as g, cY as S, cL as B, r, bq as L, bj as f, By as N, Bz as h, BA as I, BB as R, rB as P, f7 as V, vy as j, bu as k, X as D, BC as E, cy as M, co as O, BD as C } from "./reconnectSaga-3b11f62e.js";
import { S as s } from "./setting-process-8b8e42f8.js";
import "./SnackbarController-d2c726d8.js";
import "./ClickAwayListener-fa29c305.js";
import "./ListItem-f9045f06.js";
const z = ({ value: a, onChange: c, list: d, className: t, icon: m, position: o }) => {
  const { classes: n } = A();
  return e(w, { dark: false, position: o, className: n.themeSelector, toggler: (i) => p("button", { className: g(t, n.button), type: "button", children: [a, e(S, { className: g(n.arrowIcon, t, { [n.open]: i.isOpen }) })] }), children: ({ closeMenu: i }) => d.map((l) => e(B, { dense: true, onClick: () => {
    i(), c(l);
  }, className: n.themeName, endAdornment: l === a && e("div", { className: n.selectedIconCheck, children: m }), children: l }, l)) });
}, A = b()({ arrowIcon: { width: "15px", height: "6px", fill: "currentColor", marginLeft: "5px", transition: "transform 0.2s" }, open: { transform: "rotate(180deg)" }, button: { padding: "0", border: 0, background: "transparent", color: "var(--dark-grey-7)", marginRight: "-10px", fontSize: "14px", lineHeight: "18px", fontWeight: 450, textTransform: "capitalize", "&:hover": { color: "var(--accent)" } }, selectedIconCheck: { "& svg path": { fill: "var(--accent)" } }, themeName: { display: "flex", alignItems: "center", justifyContent: "space-between", textTransform: "capitalize" }, themeSelector: { minWidth: "100px", marginTop: "10px" } }), F = (a) => a.nativeTheme, q = (a, c) => r.createElement("svg", { width: 15, height: 11, viewBox: "0 0 15 11", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: c, ...a }, r.createElement("path", { d: "M13.2643 0.380859L5.125 8.52018L1.73568 5.13086L0.597656 6.26888L4.55599 10.2272L5.125 10.7715L5.69401 10.2272L14.4023 1.51888L13.2643 0.380859Z", fill: "#2091F6" })), H = r.forwardRef(q), $ = r.memo(H), W = (a) => {
  let termianVersion = O.getVersion();
  try {
    const file = require("url").fileURLToPath(new URL("../../package.json", import.meta.url));
    termianVersion = JSON.parse(require("fs").readFileSync(file, "utf8")).version;
  } catch {
  }
  // Tauri: update status from src/rust/updater.rs (window.api.updates): a
  // manual check, download progress and the restart that applies it.
  const updates = window.api && window.api.updates, { classes: u } = U();
  const [status, setStatus] = r.useState(() => updates ? updates.status() : { state: "disabled" }), [failure, setFailure] = r.useState(null);
  r.useEffect(() => updates ? updates.subscribe((t) => (setStatus(t), setFailure(null))) : void 0, []);
  const mb = (t) => (t / 1048576).toFixed(1), version = status.version ? "Termian " + status.version : "Новая версия";
  const percent = status.total ? Math.min(100, Math.round((status.downloaded || 0) * 100 / status.total)) : null;
  const checked = status.checkedAt ? " Проверено: " + new Date(status.checkedAt).toLocaleString() + "." : "";
  const text = status.state === "disabled" ? "Автообновление отключено: это локальная сборка или задан TERMIAN_DISABLE_UPDATES." : status.state === "checking" ? "Проверка обновлений…" : status.state === "downloading" ? `Загрузка ${version}: ` + (percent == null ? `${mb(status.downloaded || 0)} МБ` : `${percent}% (${mb(status.downloaded || 0)} из ${mb(status.total)} МБ)`) : status.state === "installed" ? `${version} загружена и будет установлена при перезапуске. Открытые SSH-сессии при перезапуске закроются.` : status.state === "error" ? "Не удалось проверить или загрузить обновление." + checked : status.checkedAt ? "Установлена последняя версия." + checked : "Обновления проверяются автоматически через 20 секунд после запуска и затем каждые 6 часов.";
  const busy = status.state === "checking" || status.state === "downloading", fail = (t) => setFailure(String(t && t.message || t));
  return p(j, { style: { padding: 30 }, children: p("div", { children: [e(k, { variant: "h6", color: "main", children: "Version" }), e(k, { color: "secondary", children: "termian " + termianVersion }), p("div", { className: u.updates, "data-update-state": status.state, children: [e(k, { variant: "h6", color: "main", children: "Обновления" }), e(k, { color: "secondary", role: "status", children: text }), status.state === "downloading" ? e("div", { className: u.progress, children: e("div", { className: u.progressBar, "data-indeterminate": percent == null ? "true" : "false", style: { width: percent == null ? "100%" : percent + "%" } }) }) : null, failure || status.state === "error" && status.error ? e(k, { className: u.error, children: failure || status.error }) : null, e("div", { className: u.actions, children: status.state === "installed" ? e("button", { type: "button", className: g(u.button, u.primary), onClick: () => void updates.restart().catch(fail), children: "Перезапустить и обновить" }) : e("button", { type: "button", className: u.button, disabled: !updates || busy || status.state === "disabled", onClick: () => (setFailure(null), void updates.check().catch(fail)), children: status.state === "checking" ? "Проверка…" : status.state === "downloading" ? "Загрузка…" : "Проверить обновления" }) })] })] }) });
}, U = b()({ darkThemeSwitch: { display: "flex", justifyContent: "space-between", color: "var(--dark-grey-1)", ".termius-dark-theme &": { color: "var(--white)" } }, appInfoList: { padding: "0" }, appVersion: { color: "var(--dark-grey-1)", ".termius-dark-theme &": { color: "var(--white)" } }, appVersionTab: { display: "flex", justifyContent: "flex-start" }, selectorContainer: { display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%" }, updates: { display: "flex", flexDirection: "column", gap: "8px", marginTop: "28px", maxWidth: "420px" }, progress: { height: "6px", borderRadius: "3px", overflow: "hidden", background: "var(--light-grey-transparent-10, rgba(127,127,127,.2))" }, progressBar: { height: "100%", borderRadius: "3px", background: "var(--accent, #00cc74)", transition: "width .2s", '&[data-indeterminate="true"]': { opacity: 0.5 } }, error: { color: "#ec706f", fontSize: "13px", overflowWrap: "anywhere" }, actions: { display: "flex", gap: "8px", marginTop: "4px" }, button: { minHeight: "32px", padding: "6px 14px", border: "1px solid var(--border-color, #454950)", borderRadius: "6px", background: "transparent", color: "inherit", font: "inherit", fontSize: "13px", cursor: "pointer", "&:hover:not(:disabled)": { borderColor: "var(--accent, #00cc74)" }, "&:disabled": { opacity: 0.5, cursor: "default" } }, primary: { borderColor: "var(--accent, #00cc74)", background: "var(--accent, #00cc74)", color: "#fff" } }), K = W;
export {
  K as default
};
