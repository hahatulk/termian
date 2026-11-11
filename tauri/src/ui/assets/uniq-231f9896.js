/*
 * Recovered Termius UI module: uniq-231f9896.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Report or omit repeated line", n = "uniq", i = [{ name: "input", template: ["filepaths", "folders"], isOptional: true }, { name: "output", isOptional: true }], t = [{ description: "Prefix lines by the number of occurrences", name: ["-c", "--count"] }, { description: "Only print duplicate lines", name: ["-d", "--repeated"] }, { description: "Print all duplicate lines. Delimiting is done with blank lines", name: ["-D", "--all-repeated"], args: [{ name: "delimit-method", suggestions: ["none", "prepend", "separate"], isOptional: true }] }, { description: "Avoid comparing the first N fields", name: ["-f", "--skip-fields"], args: [{ name: "number" }] }, { description: "Ignore differences in case when comparing", name: ["-i", "--ignore-case"] }, { description: "Avoid comparing the first N characters", name: ["-s", "--skip-chars"], args: [{ name: "number" }] }, { description: "Only print unique lines", name: ["-u", "--unique"] }, { description: "End lines with 0 byte, not newline", name: ["-z", "--zero-terminated"] }, { description: "Compare no more than N characters in lines", name: ["-w", "--check-chars"], args: [{ name: "number" }] }, { description: "Display this help and exit", name: "--help" }, { description: "Output version information and exit", name: "--version" }], s = { description: e, name: n, args: i, options: t };
export {
  i as args,
  s as default,
  e as description,
  n as name,
  t as options
};
