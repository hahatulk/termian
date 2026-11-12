/*
 * Recovered Termius UI module: ldd-0175e054.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Print shared library dependencies", n = "ldd", i = [{ name: "exe", template: ["filepaths"], isVariadic: true }], o = [{ description: "Show help for ldd", name: "--help" }, { description: "Print version information and exit", name: "--version" }, { description: "Process data relocations", name: ["-d", "--data-relocs"] }, { description: "Process data and function relocations", name: ["-r", "--function-relocs"] }, { description: "Print unused direct dependencies", name: ["-u", "--unused"] }, { description: "Print all information", name: ["-v", "--verbose"] }], s = { description: e, name: n, args: i, options: o };
export {
  i as args,
  s as default,
  e as description,
  n as name,
  o as options
};
