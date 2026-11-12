/*
 * Recovered Termius UI module: cot-dc82dd52.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Command-line utility for CotEditor", n = "cot", o = [{ template: ["filepaths"] }], i = [{ description: "Show help message and exit", name: ["-h", "--help"] }, { description: "Show program's version number and exit", name: ["-v", "--version"] }, { description: "Wait for opened file to be closed", name: ["-w", "--wait"] }, { description: "Do not bring the application to the foreground", name: ["-g", "--background"] }, { description: "Create a new blank document", name: ["-n", "--new"] }, { description: "Jump to specific line in opened document", name: ["-l", "--line"], args: [{ name: "line" }] }, { description: "Jump to specific column in opened document", name: ["-c", "--column"], args: [{ name: "column" }] }], t = { description: e, name: n, args: o, options: i };
export {
  o as args,
  t as default,
  e as description,
  n as name,
  i as options
};
