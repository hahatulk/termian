/*
 * Recovered Termius UI module: readlink-d77a5f65.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const t = "Display file status", e = "readlink", a = [{ name: "file", description: "File(s) to readlink", template: ["filepaths"], isVariadic: true }], i = [{ description: "Display information using the specified format; similar to printf(3) formats in that they start with %, are then followed by a sequence of formatting characters, and end in a character that selects the field of the struct stat which is to be formatted", name: "-f", args: [{ name: "format" }] }, { description: "Do not force a newline to appear at the end of each piece of output", name: "-n" }], n = { description: t, name: e, args: a, options: i };
export {
  a as args,
  n as default,
  t as description,
  e as name,
  i as options
};
