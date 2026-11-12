/*
 * Recovered Termius UI module: remix-79a5a614.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Remix CLI to start, build and export your application", t = "remix", i = [{ description: "Create an optimized production build of your application", name: "build", args: [{ name: "dir", description: "Represent the directory of the Remix application", template: ["folders"], isOptional: true }], options: [{ description: "Enables production sourcemap", name: "--sourcemap" }] }, { description: "Start the application in development mode", name: "dev", args: [{ name: "dir", description: "Represent the directory of the Remix application", template: ["folders"], isOptional: true }] }, { description: "Prepare node_modules/remix folder (after installation of packages)", name: "setup", args: [{ name: "dir", description: "Represent the directory of the Remix application", template: ["folders"], isOptional: true }] }, { description: "Generate the route config of the application", name: "routes", args: [{ name: "dir", description: "Represent the directory of the Remix application", template: ["folders"], isOptional: true }], options: [{ description: "Print the route config as JSON", name: "--json" }] }], o = [{ description: "Output usage information", name: "--help" }, { description: "Output the version number", name: ["-v", "--version"] }], n = { description: e, name: t, subcommands: i, options: o };
export {
  n as default,
  e as description,
  t as name,
  o as options,
  i as subcommands
};
