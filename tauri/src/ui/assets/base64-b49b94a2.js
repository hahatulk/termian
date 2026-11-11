/*
 * Recovered Termius UI module: base64-b49b94a2.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Encode and decode using Base64 representation", t = "base64", s = [{ description: "Display this help and exit", name: ["--help", "-h"] }, { description: "Insert line breaks every count characters.  Default is 0, which generates an unbroken stream", name: ["--break", "-b"], args: [{ name: "count", suggestions: ["0"] }] }, { description: "Decode incoming Base64 stream into binary data", name: ["--decode", "-d", "-D"] }, { description: "Read input from input_file.  Default is stdin; passing - also represents stdin", name: ["--input", "-i"], args: [{ name: "input_file", suggestions: ["stdin", "-"], template: ["filepaths"] }] }, { description: "Write output to output_file.  Default is stdout; passing - also represents stdout", name: ["--output", "-o"], args: [{ name: "output_file", suggestions: ["stdout", "-"], template: ["filepaths"] }] }], n = { optionsMustPrecedeArguments: true }, i = { description: e, name: t, options: s, parserDirectives: n };
export {
  i as default,
  e as description,
  t as name,
  s as options,
  n as parserDirectives
};
