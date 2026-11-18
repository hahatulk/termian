/*
 * Recovered Termius UI module: wing-d12a7f40.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "wing", n = [{ description: "Runs a Wing executable in the Wing Console", name: "run", args: [{ name: "executable", description: "Executable .wx file" }], options: [{ description: "Display help for command", name: ["-h", "--help"] }] }, { description: "Compiles a Wing program", name: "compile", args: [{ name: "entrypoint", description: "Program .w entrypoint" }], options: [{ description: "Output directory", name: ["-o", "--out-dir"], args: [{ name: "out-dir" }] }, { description: "Target platform (options: 'tf-aws', 'sim')", name: ["-t", "--target"], args: [{ name: "target" }] }, { description: "Display help for command", name: ["-h", "--help"] }] }, { description: "Upgrades the Wing toolchain to the latest version", name: "upgrade", options: [{ description: "Display help for command", name: ["-h", "--help"] }] }, { description: "Display help for command", name: "help", args: [{ name: "command", template: ["help"], isOptional: true }] }], a = { name: e, subcommands: n };
export {
  a as default,
  e as name,
  n as subcommands
};
