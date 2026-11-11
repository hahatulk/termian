/*
 * Recovered Termius UI module: rm-8e7d66f8.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Remove directory entries", t = "rm", i = [{ template: ["folders", "filepaths"], isVariadic: true }], o = [{ description: "Recursive. Attempt to remove the file hierarchy rooted in each file argument", name: ["-r", "-R"] }, { description: "Overwrite regular files before deleting them", name: "-P" }, { description: "Attempt to remove directories as well as other types of files", name: "-d" }, { description: "⚠️ Attempt to remove the files without prompting for confirmation", name: "-f" }, { description: "Request confirmation before attempting to remove each file", name: "-i" }, { description: "Be verbose when deleting files", name: "-v" }], r = { description: e, name: t, args: i, options: o };
export {
  i as args,
  r as default,
  e as description,
  t as name,
  o as options
};
