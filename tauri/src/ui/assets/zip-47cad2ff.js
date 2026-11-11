/*
 * Recovered Termius UI module: zip-47cad2ff.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Package and compress (archive) files into zip file", s = "zip", a = [{ name: "name", description: "Name of archive" }, { name: "dir", template: ["folders"] }], i = [{ description: "Package and compress a directory and its contents, recursively", name: "-r" }, { name: "-e" }, { name: "-s", args: [{ name: "split size" }] }, { name: "-d", args: [{ name: "file", template: ["filepaths"] }] }, { description: "Archive a directory and its contents with the highest level [9] of compression", name: "-9" }], n = { description: e, name: s, args: a, options: i };
export {
  a as args,
  n as default,
  e as description,
  s as name,
  i as options
};
