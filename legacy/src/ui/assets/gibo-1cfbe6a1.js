/*
 * Recovered Termius UI module: gibo-1cfbe6a1.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Easy access to gitignore boilerplates", o = "gibo", a = [{ description: "Dump one or more boilerplates to STDOUT", name: "dump", args: [{ name: "boilerplate", description: "Name of the boilerplate to dump", isVariadic: true }] }, { description: "Search for boilerplates", name: "search", args: [{ name: "search term", description: "Name of the language or framework to search for" }] }], r = [{ description: "Show help for gibo", name: ["--help", "-h"] }], s = { description: e, name: o, subcommands: a, options: r };
export {
  s as default,
  e as description,
  o as name,
  r as options,
  a as subcommands
};
