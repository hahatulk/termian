/*
 * Recovered Termius UI module: find-e808cd3b.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Walk a file hierarchy", i = "find", n = [{ name: "path", template: ["folders"], isVariadic: true, isOptional: true }, { name: "expression", description: "Composition of primaries and operands", isVariadic: true, isOptional: true }], t = [{ description: "Interpret regular expressions followed by -regex and -iregex primaries as extended", name: "-E" }, { description: "Cause the file information and file type returned for each symbolic link specified to be those referenced by the link", name: "-H" }, { description: "Cause the file information and file type returned for each symbolic link to be those of the file referenced by the link", name: "-L" }, { description: "Cause the file information and file type returned for each symbolic link to be those for the link itself", name: "-P" }, { description: "Permit find to be safely used in conjunction with xargs", name: "-X" }, { description: "Cause find to perform a depth-first traversal", name: "-d" }, { description: "Specify a file hierarch for find to traverse", name: "-f", args: [{ name: "path" }] }, { description: "Cause find to traverse the file hierarchies in lexicographical order", name: "-s" }, { description: "Prevent find from descending into directories that have a device number different than that of the file from which the descent began", name: "-x" }], r = { description: e, name: i, args: n, options: t };
export {
  n as args,
  r as default,
  e as description,
  i as name,
  t as options
};
