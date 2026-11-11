/*
 * Recovered Termius UI module: cut-18fcf035.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Cut out selected portions of each line of a file", i = "cut", s = [{ template: ["filepaths"], isVariadic: true, isOptional: true }], t = [{ description: "Byte positions as a comma or - separated list of numbers", name: "-b", args: [{ name: "list", description: "Specifies byte positions" }] }, { description: "Column positions as a comma or - separated list of numbers", name: "-c", args: [{ name: "list", description: "Specifies column positions" }] }, { description: "Field positions as a comma or - separated list of numbers", name: "-f", args: [{ name: "list", description: "Specifies column positions" }] }, { description: "Do not split multi-byte characters", name: "-n" }, { description: "Use delim as the field delimiter character instead of the tab character", name: "-d", args: [{ name: "delim", description: "Field deliminator to use instead of the tab character", isOptional: true }] }, { description: "Suppress lines with no field delimiter characters.  unless specified, lines with no delimiters are passed through unmodified", name: "-s" }], o = { description: e, name: i, args: s, options: t };
export {
  s as args,
  o as default,
  e as description,
  i as name,
  t as options
};
