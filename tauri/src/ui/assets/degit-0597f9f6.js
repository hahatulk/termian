/*
 * Recovered Termius UI module: degit-0597f9f6.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Straightforward project scaffolding", o = "degit", t = [{ name: "user/repo" }, { name: "location", template: ["folders"], isOptional: true }], n = [{ description: "Print help", name: "--help" }, { description: "Overwrite existing files", name: ["-f", "--force"] }, { description: "Use a cache", name: ["-c", "--cache"] }, { description: "Be verbose?", name: ["-v", "--verbose"] }, { description: "Clone mode", name: ["-m", "--mode"], args: [{ name: "mode", suggestions: ["git", "tar"] }] }], i = { description: e, name: o, args: t, options: n };
export {
  t as args,
  i as default,
  e as description,
  o as name,
  n as options
};
