/*
 * Recovered Termius UI module: seq-2bf18eb5.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Print sequences of numbers. (Defaults to increments of 1)", n = "seq", t = [{ name: "first", description: "Starting number in sequence" }, { name: "step", description: "Increment interval", isOptional: true }, { name: "last", description: "Last number in sequence", isOptional: true }], s = [{ description: "Equalize the widths of all numbers by padding with zeros as necessary", name: ["-w", "--fixed-width"] }, { insertValue: '-s "{cursor}"', description: "String separator between numbers. Default is newline", name: ["-s", "--separator"], args: [{ name: "string", description: "Separator" }] }, { insertValue: "-f %{cursor}", description: "Use a printf(3) style format to print each number", name: ["-f", "--format"], args: [{ name: "format", description: "Print all numbers using format" }] }, { insertValue: '-t "{cursor}"', description: "Use string to terminate sequence of numbers", name: ["-t", "--terminator"], args: [{ name: "string", description: "Terminator" }] }], r = { description: e, name: n, args: t, options: s };
export {
  t as args,
  r as default,
  e as description,
  n as name,
  s as options
};
