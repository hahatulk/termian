/*
 * Recovered Termius UI module: chsh-2870b16d.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Change your login shell", s = "chsh", n = [{ name: "username", description: "Target user", isOptional: true }], i = [{ description: "Specify login shell", name: ["-s", "--shell"], args: [{ name: "shell" }] }, { description: "Print list of shells and exit", name: ["-l", "--list-shells"] }, { description: "Print help message and exit", name: ["-u", "-h", "--help"] }, { description: "Print version and exit", name: ["-v", "--version"] }], t = { description: e, name: s, args: n, options: i };
export {
  n as args,
  t as default,
  e as description,
  s as name,
  i as options
};
