/*
 * Recovered Termius UI module: bcd-80995dce.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Bookmark directories and move to them", o = "bcd", t = [{ name: "bookmark", isOptional: true }], i = [{ description: "Store the current directory as a bookmark", name: ["-s", "--store"], args: [{ name: "store", isOptional: true }] }, { description: "Remove a specified bookmark", name: ["-r", "--remove"], args: [{ name: "remove", isOptional: true }] }, { description: "Setup the the shell init script", name: ["-i", "--install"] }, { description: "List the stored bookmarks", name: ["-l", "--list"] }, { description: "Print version information", name: ["-V", "--version"] }, { description: "Print help information", name: ["-h", "--help"] }], n = { description: e, name: o, args: t, options: i };
export {
  t as args,
  n as default,
  e as description,
  o as name,
  i as options
};
