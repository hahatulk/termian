/*
 * Recovered Termius UI module: vr-b83462c8.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "The npm-style script runner for Deno", r = "vr", o = [{ description: "Run a script", name: "run", args: [{ name: "script" }] }, { description: "Export one or more scripts as standalone executable files", name: "export", args: [{ name: "script", isVariadic: true }] }, { description: "Upgrade Velociraptor to the latest version or to a specific one", name: "upgrade", args: [{ name: "version", isOptional: true }], options: [{ description: "The folder where the scripts will be exported", name: ["-o", "--out-dir"], args: [{ name: "dir" }] }] }], s = [{ name: "script" }], n = [{ description: "Show help for Velociraptor", name: ["--help", "-h"] }, { description: "Show the version number for Velociraptor", name: ["-V", "--version"] }], t = { description: e, name: r, subcommands: o, args: s, options: n };
export {
  s as args,
  t as default,
  e as description,
  r as name,
  n as options,
  o as subcommands
};
