/*
 * Recovered Termius UI module: hx-0be3583f.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "A post-modern text editor", t = "hx", i = [{ name: "files", template: ["filepaths", "folders"], isVariadic: true }], r = [{ description: "Show help", name: ["-h", "--help"] }, { description: "Open the tutorial", name: "--tutor" }, { description: "Check for errors in editor setup", name: "--health", args: [{ name: "language", isOptional: true }] }, { description: "Increases logging verbosity", name: "-v" }, { description: "Fetch or build tree-sitter grammars", name: ["-g", "--grammar"], args: [{ name: "action", suggestions: [{ name: "fetch" }, { name: "build" }] }] }, { description: "Print version information", name: ["-V", "--version"] }], n = { flagsArePosixNoncompliant: true }, s = { description: e, name: t, args: i, options: r, parserDirectives: n };
export {
  i as args,
  s as default,
  e as description,
  t as name,
  r as options,
  n as parserDirectives
};
