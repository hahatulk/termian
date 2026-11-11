/*
 * Recovered Termius UI module: zed-25542e4d.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "A lightning-fast, collaborative code editor written in Rust", t = "zed", a = [{ name: "path", description: "A sequence of space-separated paths that you want to open", template: ["filepaths"], isVariadic: true, isOptional: true }], n = [{ description: "Print help information", name: ["-h", "--help"] }, { description: "Print Zed's version and the app path", name: ["-v", "--version"] }, { description: "Wait for all of the given paths to be closed before exiting", name: ["-w", "--wait"] }, { description: "Custom Zed.app path", name: ["-b", "--bundle-path"], args: [{ name: "bundle_path", template: ["folders"] }] }], i = { description: e, name: t, args: a, options: n };
export {
  a as args,
  i as default,
  e as description,
  t as name,
  n as options
};
