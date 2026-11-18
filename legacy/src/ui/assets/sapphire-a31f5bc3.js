/*
 * Recovered Termius UI module: sapphire-a31f5bc3.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "CLI for the Sapphire Framework", n = "sapphire", a = [{ displayName: "Create a new Sapphire project 📂", description: "Creates a new Sapphire project", name: ["new", "n"], args: [{ name: "name", description: "Project name", isOptional: true }], options: [{ description: "Enable verbose logging", name: ["-v", "--verbose"] }] }, { displayName: "Generate a component/piece 📄", description: "Generates a component/piece", name: ["generate", "g"], args: [{ name: "component", description: "Component/piece name" }, { name: "name", description: "File name", template: ["filepaths"] }] }, { displayName: "Display help for a command 📖", description: "Display help for command", name: "help", args: [{ name: "command", description: "Command to display help for", template: ["help"], isOptional: true }] }], o = [{ description: "Show help for sapphire", name: ["--help", "-h"] }, { description: "Output the version number", name: ["--version", "-V"] }], p = { description: e, name: n, subcommands: a, options: o };
export {
  p as default,
  e as description,
  n as name,
  o as options,
  a as subcommands
};
