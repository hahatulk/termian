/*
 * Recovered Termius UI module: arch-3f6cb427.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Print architecture type or run select architecture", t = "arch", i = [{ name: "program", template: ["filepaths"], isVariadic: true, isCommand: true }], n = [{ description: "Add the native 32-bit architecture to the list of architectures", name: "-32" }, { description: "Add the native 64-bit architecture to the list of architectures", name: "-64" }, { name: "-arch", args: [{ name: "arch_name", suggestions: [{ description: "32-bit intel", name: "i386" }, { description: "64-bit intel", name: "x86_64" }, { description: "64-bit intel (haswell)", name: "x86_64h" }, { description: "64-bit arm", name: "arm64" }, { description: "64-bit arm (Apple Silicon)", name: "arm64e" }] }] }, { description: "32-bit intel", name: "-i386" }, { description: "64-bit intel", name: "-x86_64" }, { description: "64-bit intel (haswell)", name: "-x86_64h" }, { description: "64-bit arm", name: "-arm64" }, { description: "64-bit arm (Apple Silicon)", name: "-arm64e" }, { description: "Clear the environment that will be passed to the command", name: "-c" }, { description: "Delete the named environment variable from the command's environment", name: "-d", args: [{ name: "envname" }] }, { description: "Assign the given value to the variable in the command's environment", name: "-e", args: [{ name: "envname=value" }] }, { description: "Print a help message and exit", name: "-h" }], a = { flagsArePosixNoncompliant: true, optionsMustPrecedeArguments: true }, r = { description: e, name: t, args: i, options: n, parserDirectives: a };
export {
  i as args,
  r as default,
  e as description,
  t as name,
  n as options,
  a as parserDirectives
};
