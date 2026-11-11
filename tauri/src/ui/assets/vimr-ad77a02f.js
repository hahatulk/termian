/*
 * Recovered Termius UI module: vimr-ad77a02f.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "VimR — Neovim GUI for macOS in Swift", n = "vimr", i = [{ template: ["filepaths", "folders"] }], o = [{ description: "Show help", name: ["-h", "--help"] }, { description: "Just print the 'open' command", name: "--dry-run" }, { description: "Set the working directory", name: "--cwd", args: [{ name: "CWD" }] }, { description: "Go to line", name: "--line", args: [{ name: "LINE" }] }, { description: "This command line tool will exit when the corresponding UI window is closed", name: "--wait" }, { description: "All arguments except --cur-env, --line, --dry-run and --wait will be passed over to the (new) nvim instance in a new UI window", name: "--nvim" }, { description: "Use the current environment variables when launching the background neovim process. All files will be opened in a new window", name: "--cur-env" }, { description: "Open files in tabs in a new window", name: "-n" }, { description: "Open files in separate windows", name: "-s" }], t = { description: e, name: n, args: i, options: o };
export {
  i as args,
  t as default,
  e as description,
  n as name,
  o as options
};
