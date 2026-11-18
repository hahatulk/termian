/*
 * Recovered Termius UI module: deployctl-5c254577.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Command line tool for Deno Deploy", t = "deployctl", n = [{ description: "Deploy a script with static files to Deno Deploy", name: "deploy", args: [{ name: "entrypoint" }], options: [{ insertValue: "--exclude '{cursor}'", description: "Exclude files that match this pattern", name: "--exclude", args: [{ name: "pattern" }] }, { insertValue: "--include '{cursor}'", description: "Only upload files that match this pattern", name: "--include", args: [{ name: "pattern" }] }, { description: "Don't include the files in the CWD as static files", name: "--no-static" }, { description: "Create a production deployment (default is preview deployment)", name: "--prod" }, { description: "The project to deploy to", name: ["-p", "--project"], args: [{ name: "name" }] }, { description: "The API token to use", name: "--token", args: [{ name: "token" }] }] }, { description: "Upgrade deployctl", name: "upgrade", args: [{ name: "version", isOptional: true }] }], o = [{ description: "Show help", name: ["--help", "-h"] }, { description: "Show the version", name: ["-V", "--version"] }], i = { description: e, name: t, subcommands: n, options: o };
export {
  i as default,
  e as description,
  t as name,
  o as options,
  n as subcommands
};
