/*
 * Recovered Termius UI module: kubectx-be453198.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Switch between Kubernetes-contexts", t = "kubectx", n = [{ name: "context", isOptional: true }], o = [{ description: "Show help for kubectx", name: ["--help", "-h"] }, { description: "Show current context", name: ["--current", "-c"] }, { description: "Unset the current context", name: ["--unset", "-u"] }, { description: "Delete context", name: "-d", args: [{ name: "context", isVariadic: true }] }], s = [{ description: "Switch to previous context", name: "-" }], i = { flagsArePosixNoncompliant: true }, c = { description: e, name: t, args: n, options: o, additionalSuggestions: s, parserDirectives: i };
export {
  s as additionalSuggestions,
  n as args,
  c as default,
  e as description,
  t as name,
  o as options,
  i as parserDirectives
};
