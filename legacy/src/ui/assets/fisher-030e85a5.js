/*
 * Recovered Termius UI module: fisher-030e85a5.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const i = "A plugin manager for Fish", e = "fisher", n = [{ description: "Remove plugins", name: "remove", args: [{ name: "installed plugins", description: "The plugin you want to remove", isVariadic: true }] }, { description: "Update plugins", name: "update", args: [{ name: "installed plugins", description: "The plugin you want to update", isVariadic: true }] }, { description: "List plugins", name: "list", args: [{ name: "RegEx", description: "Search in list with regular expression", isOptional: true }] }], s = [{ description: "Show help for fisher", name: ["--help", "-h"] }, { description: "Show fisher version", name: ["--version", "-v"] }], o = { description: i, name: e, subcommands: n, options: s };
export {
  o as default,
  i as description,
  e as name,
  s as options,
  n as subcommands
};
