/*
 * Recovered Termius UI module: yank-159fb511.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Yank terminal output to clipboard", n = "yank", t = [{ description: "Ignore case differences between pattern and the input", name: "-i" }, { description: "Use the default delimiters except for space", name: "-l" }, { description: "Use alternate screen", name: "-x" }, { description: "Print the version", name: "-v" }, { description: "All input characters not present in delim will be recognized as fields", name: "-d", args: [{ name: "delim", description: "Custom delimiters" }] }, { description: "Use pattern to recognize fields", name: "-g", args: [{ name: "pattern", description: "Pattern to recognize fields" }] }, { description: "Use a command as the yank command", name: "--", args: [{ name: "command", description: "Command to use as the yank command", isVariadic: true }] }], i = { description: e, name: n, options: t };
export {
  i as default,
  e as description,
  n as name,
  t as options
};
