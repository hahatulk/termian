/*
 * Recovered Termius UI module: DeveloperTools-e07b2297.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { zc as g, ay as p, r as c, br as i, bt as a, cy as k, ms as U, dl as B, cv as b, bi as x, Cm as A, Cn as T, Ak as E, Am as y, Co as D } from "./reconnectSaga-3b11f62e.js";
import { S as I } from "./SettingsTabContent-ce30872f.js";
import { S } from "./SettingsSwitch-d35e9651.js";
import { S as v, a as f } from "./SettingsSection-bc44d6bf.js";
const C = { dev: "https://dev.api.termius.com", prod: "https://api.termius.com", local: "http://api.termius.localhost" }, L = () => {
  const n = g(), t = "https://api.termius.com", e = !!p.getDeprecatedCustomBackendApiUrl(), [l, o] = c.useState(t);
  c.useEffect(() => {
    o(e ? p.getDeprecatedCustomBackendApiUrl() || "" : t);
  }, []);
  const m = c.useCallback((d) => {
    o(d.target.value);
  }, []), r = c.useCallback((d) => {
    p.setDeprecatedCustomBackendApiUrl(d);
  }, []), h = c.useCallback(() => {
    if (e) {
      p.resetDeprecatedCustomBackendApiUrl(), o(t), n();
      return;
    }
    r(l), n();
  }, [l, e, r, n, t]), s = c.useCallback((d) => {
    r(d), o(d);
  }, [r]), u = c.useCallback(() => {
    if (!l) {
      s(t);
      return;
    }
    r(l);
  }, [l, t, s, r]);
  return i(v, { children: [a(f, { children: "API Settings" }), a(S, { checked: e, label: "Enable custom backend API (restart required)", onChange: h }), a(k, { top: 10, children: a(U, { type: "text", label: "Backend URL", fullWidth: true, value: l, disabled: !e, onChange: m, onBlur: u }) }), a(k, { top: 15, children: i(B, { children: [a(b, { disabled: !e, size: "small", onClick: () => s(C.dev), children: "Development" }), a(b, { disabled: !e, size: "small", onClick: () => s(C.prod), children: "Production" }), a(b, { disabled: !e, size: "small", onClick: () => s(C.local), children: "Local" })] }) })] });
}, V = (n) => {
  const { label: t, name: e, defaultValue: l, availableValues: o, onChange: m } = n, { classes: r } = j(), h = (s) => {
    const { target: u } = s;
    m(u.value);
  };
  return a(k, { top: 10, children: i("div", { className: r.container, children: [a("span", { children: t }), i("div", { children: [a(T, { id: `experiment-label-${e}` }), a(E, { labelId: `experiment-label-${e}`, id: `experiment-select-${e}`, value: l, onChange: h, children: o.map((s, u) => a(y, { value: s, children: s }, String(s + u))) })] })] }) });
}, j = x()({ container: { display: "flex", justifyContent: "space-between", alignItems: "center", width: "100%" } });
function w() {
  const n = g(), t = c.useCallback((e, l) => {
    e.update(l), n();
  }, [n]);
  return i(v, { children: [a(f, { children: "Experiment Toggles" }), Object.values(A).map((e) => a(V, { label: e.name, name: e.name, defaultValue: e.value(), availableValues: e.availableValues, onChange: (l) => {
    t(e, l);
  } }, e.name))] });
}
function z() {
  const n = g(), t = c.useCallback((e) => {
    e.update(!e.value()), n();
  }, [n]);
  return i(v, { children: [a(f, { children: "Feature Toggles" }), Object.values(D).map((e) => a(S, { checked: e.value(), label: e.label, onChange: () => t(e) }, e.label))] });
}
const M = () => i(I, { children: [a(z, {}), a(w, {}), a(L, {})] });
export {
  M as default
};
