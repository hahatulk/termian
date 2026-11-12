/*
 * Recovered Termius UI module: omz-7d5fa250.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Oh My Zsh", n = "omz", a = [{ description: "Manage plugins", name: "plugin", subcommands: [{ description: "Disable plugin(s)", name: "disable", args: [{ name: "plugin" }] }, { description: "Enable plugin(s)", name: "enable", args: [{ name: "plugin" }] }, { description: "Get information of a plugin", name: "info", args: [{ name: "plugin" }] }, { description: "Load plugin(s)", name: "load", args: [{ name: "plugin" }] }] }, { description: "Manage Oh My Zsh Pull Requests", name: "pr", subcommands: [{ description: "Fetch PR #NUMBER and rebase against master", name: "test", args: [{ name: "PR_number_or_URL" }] }] }, { description: "Manage themes", name: "theme", subcommands: [{ description: "Set a theme in your .zshrc file", name: "set", args: [{ name: "theme" }] }, { description: "Load a theme", name: "use", args: [{ name: "theme" }] }] }], s = { description: e, name: n, subcommands: a };
export {
  s as default,
  e as description,
  n as name,
  a as subcommands
};
