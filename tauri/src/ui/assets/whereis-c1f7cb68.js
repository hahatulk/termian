/*
 * Recovered Termius UI module: whereis-c1f7cb68.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Locate the binary, source, and manual page files for a command", r = "whereis", i = [{ name: "Filename", description: "The file to search for" }], o = [{ description: "Search only for binaries", name: "-b" }, { description: "Search only for manual sections", name: "-m" }, { description: "Search only for sources", name: "-s" }, { description: "Search for unusual entries", name: "-u" }, { description: "Search for binaries only in the specified directory", name: "-B", args: [{ name: "directory", description: "The directory to search in", template: ["folders"] }] }, { description: "Search for manual pages only in the specified directory", name: "-M", args: [{ name: "directory", description: "The directory to search in", template: ["folders"] }] }, { description: "Search for sources only in the specified directory", name: "-S", args: [{ name: "directory", description: "The directory to search in", template: ["folders"] }] }, { description: "Terminate the -B, -M, and -S options", name: "-f" }], n = { description: e, name: r, args: i, options: o };
export {
  i as args,
  n as default,
  e as description,
  r as name,
  o as options
};
