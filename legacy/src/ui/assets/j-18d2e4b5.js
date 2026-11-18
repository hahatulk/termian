/*
 * Recovered Termius UI module: j-18d2e4b5.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "A faster way to navigate your filesystem", t = "autojump", i = [{ name: "directory", description: "Directory to jump to", isVariadic: true }], a = [{ description: "Show the help message and exit", name: ["-h", "--help"] }, { description: "Add path", name: ["-a", "--add"], args: [{ name: "directory", template: ["folders"] }] }, { description: "Increase current directory weight", name: ["-i", "--increase"], args: [{ name: "weight", isOptional: true }] }, { description: "Decrease current directory weight", name: ["-d", "--decrease"], args: [{ name: "weight", isOptional: true }] }, { description: "Used for tab completion", name: "--complete" }, { description: "Remove non-existent paths from database", name: "--purge" }, { description: "Show database entries and their key weights", name: ["-s", "--stat"] }, { description: "Show version information", name: ["-v", "--version"] }], n = { description: e, name: t, args: i, options: a };
export {
  i as args,
  n as default,
  e as description,
  t as name,
  a as options
};
