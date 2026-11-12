/*
 * Recovered Termius UI module: hyper-5b491790.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const n = "Hyper is an Electron-based terminal", e = "hyper", s = [{ description: "Install a plugin", name: ["install", "i"], args: [{ name: "plugin" }] }, { description: "Open the npm page of a plugin", name: ["docs", "d", "h", "home"], args: [{ name: "plugin" }] }, { description: "Search for plugins on npm", name: ["search", "s"], args: [{ name: "query", description: "Your search query", isOptional: true }] }, { description: "Uninstall plugin", name: ["uninstall", "u", "rm", "remove"], args: [{ name: "plugin", description: "Plugin to uninstall" }] }], a = [{ template: ["folders"] }], i = [{ description: "Output usage information", name: ["-h", "--help"] }, { description: "Verbose mode (disabled by default)", name: ["-v", "--verbose"] }], o = { description: n, name: e, subcommands: s, args: a, options: i };
export {
  a as args,
  o as default,
  n as description,
  e as name,
  i as options,
  s as subcommands
};
