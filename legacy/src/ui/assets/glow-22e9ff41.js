/*
 * Recovered Termius UI module: glow-22e9ff41.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Render markdown on the CLI, with pizzazz!", o = "glow", a = [{ description: "Help about any command", name: "help", args: [{ name: "command", template: ["help"], isOptional: true }] }, { description: "Manage your stash of markdown files", name: "stash", args: [{ name: "path", isOptional: true }], options: [{ insertValue: "--memo '{cursor}'", description: "Memo/note for stashing", name: ["-m", "--memo"], args: [{ name: "note" }] }] }], n = [{ name: "source|dir", isVariadic: true, isOptional: true }], t = [{ description: "Show system files and directories (TUI-mode only)", name: ["-a", "--all"] }, { description: "Config file", name: "--config", args: [{ name: "path", template: ["filepaths"] }] }, { description: "Help for glow", name: ["-h", "--help"] }, { description: "Show local files only; no network (TUI-mode only)", name: ["-l", "--local"] }, { description: "Display with pager", name: ["-p", "--pager"] }, { description: "Style name or JSON path (default 'auto')", name: ["-s", "--style"], args: [{ name: "name", suggestions: ["auto", "light", "dark", "notty"], template: ["filepaths"] }] }, { description: "Version for glow", name: ["-v", "--version"] }, { description: "Word-wrap at width", name: ["-w", "--width"], args: [{ name: "column" }] }], s = { description: e, name: o, subcommands: a, args: n, options: t };
export {
  n as args,
  s as default,
  e as description,
  o as name,
  t as options,
  a as subcommands
};
