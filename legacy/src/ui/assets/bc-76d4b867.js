/*
 * Recovered Termius UI module: bc-76d4b867.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "An arbitrary precision calculator language", t = "bc", i = [{ name: "file", template: ["filepaths"], isVariadic: true, isOptional: true }], n = [{ description: "Print the usage and exit", name: ["--help", "-h"] }, { description: "Force interactive mode", name: ["--interactive", "-i"] }, { description: "Define the standard math library", name: ["--mathlib", "-l"] }, { description: "Give warnings for extensions to POSIX bc", name: ["--warn", "-w"] }, { description: "Process exactly the POSIX bc language", name: ["--standard", "-s"] }, { description: "Do not print the normal GNU bc welcome", name: ["--quiet", "-q"] }, { description: "Print the version number and copyright and quit", name: ["--version", "-v"] }], r = { optionsMustPrecedeArguments: true }, a = { description: e, name: t, args: i, options: n, parserDirectives: r };
export {
  i as args,
  a as default,
  e as description,
  t as name,
  n as options,
  r as parserDirectives
};
