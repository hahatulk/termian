/*
 * Recovered Termius UI module: kubens-e8298aa0.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Switch between Kubernetes-namespaces", n = "kubens", s = [{ name: "namespace", isOptional: true }], t = [{ description: "Show help for kubens", name: ["--help", "-h"] }, { description: "Show current namespace", name: ["--current", "-c"] }], i = [{ description: "Switch to previous namespace within the current context", name: "-" }], o = { flagsArePosixNoncompliant: true }, a = { description: e, name: n, args: s, options: t, additionalSuggestions: i, parserDirectives: o };
export {
  i as additionalSuggestions,
  s as args,
  a as default,
  e as description,
  n as name,
  t as options,
  o as parserDirectives
};
