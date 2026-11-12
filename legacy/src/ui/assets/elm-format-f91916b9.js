/*
 * Recovered Termius UI module: elm-format-f91916b9.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Format your code in the Elm idiomatic way", t = "elm-format", o = [{ name: "INPUT", template: ["filepaths"] }], i = [{ description: "Show help for elm-format", name: ["--help", "-h"] }, { description: "Write output to FILE instead of overwriting the given source file", name: "--output", args: [{ name: "FILE", description: "Name and location of output file", template: ["filepaths"] }] }, { description: "Reply 'yes' to all automated prompts", name: "--yes" }, { description: "Check if files are formatted without changing them", name: "--validate" }, { description: "Read from stdin, output to stdout", name: "--stdin" }, { description: "The Elm version of the source files being formatted", name: "--elm-version", args: [{ name: "VERSION", description: "Valid values: 0.18, 0.19. Default: auto", isOptional: true }] }], a = { description: e, name: t, args: o, options: i };
export {
  o as args,
  a as default,
  e as description,
  t as name,
  i as options
};
