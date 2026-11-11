/*
 * Recovered Termius UI module: create-completion-spec-c138f4ea.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Setup fig folder and create spec with the given name", n = "create-completion-spec", o = [{ description: "Display help for command", name: "help", args: [{ name: "command", isOptional: true }] }], t = [{ name: "name" }], s = [{ description: "Set if the spec should be created in the current folder", name: "--here" }, { description: "Display help for command", name: ["-h", "--help"] }], a = { description: e, name: n, subcommands: o, args: t, options: s };
export {
  t as args,
  a as default,
  e as description,
  n as name,
  s as options,
  o as subcommands
};
