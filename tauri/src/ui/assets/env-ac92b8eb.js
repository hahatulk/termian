/*
 * Recovered Termius UI module: env-ac92b8eb.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Set environment and execute command, or print environment", n = "env", t = [{ name: "name=value ...", description: "Set environment variables", isOptional: true }, { name: "utility", description: "Utility to run", isOptional: true, isCommand: true }], i = [{ description: "End each output line with NUL, not newline", name: "-0" }, { description: "Start with an empty environment", name: ["-i", "-"] }, { description: "Print verbose logs", name: "-v" }, { description: "Remove variable from the environment", name: "-u", args: [{ name: "name" }] }, { description: "Search the given directories for the utility, rather than the PATH", name: "-P", args: [{ name: "altpath", template: ["folders"] }] }, { description: "Split the given string into separate arguments", name: "-S", args: [{ name: "string" }] }], a = { description: e, name: n, args: t, options: i };
export {
  t as args,
  a as default,
  e as description,
  n as name,
  i as options
};
