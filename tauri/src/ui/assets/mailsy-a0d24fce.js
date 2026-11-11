/*
 * Recovered Termius UI module: mailsy-a0d24fce.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "⚡️ Quickly generate a disposable email straight from terminal", n = "mailsy", o = [{ description: "Get help for a command", name: "help", args: [{ name: "command", template: ["help"], isOptional: true }] }], s = [{ description: "Output the current version", name: ["-v", "--version"] }, { description: "Display help for command", name: ["-h", "--help"] }], t = { description: e, name: n, subcommands: o, options: s };
export {
  t as default,
  e as description,
  n as name,
  s as options,
  o as subcommands
};
