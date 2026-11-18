/*
 * Recovered Termius UI module: where-506a47c0.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "For each name, indicate how it should be interpreted", n = "where", t = [{ name: "names", isVariadic: true }], a = [{ description: "For each name, print 'name: word', where 'word' is the kind of command", name: "-w" }, { description: "Do a path search for the name, even if it's an alias/function/builtin", name: "-p" }, { description: "The arguments are taken as patterns (pattern characters must be quoted)", name: "-m" }, { description: "If the pathname contains symlinks, print the symlink-free name as well", name: "-s" }, { description: "Print intermediate symlinks and the resolved name", name: "-S" }, { description: "Expand tabs when outputting shell function", name: "-x", args: [{ name: "num" }] }], i = { description: e, name: n, args: t, options: a };
export {
  t as args,
  i as default,
  e as description,
  n as name,
  a as options
};
