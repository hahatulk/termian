/*
 * Recovered Termius UI module: mv-ef4c5003.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Move & rename files and folders", t = "mv", o = [{ name: "source", template: ["filepaths", "folders"], isVariadic: true }, { name: "target", template: ["filepaths", "folders"] }], i = [{ description: "Do not prompt for confirmation before overwriting the destination path", name: "-f" }, { description: "Cause mv to write a prompt to standard error before moving a file that would overwrite an existing file", name: "-i" }, { description: "Do not overwrite existing file", name: "-n" }, { description: "Cause mv to be verbose, showing files after they are moved", name: "-v" }], n = { description: e, name: t, args: o, options: i };
export {
  o as args,
  n as default,
  e as description,
  t as name,
  i as options
};
