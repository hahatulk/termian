/*
 * Recovered Termius UI module: tfenv-8b22c4eb.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Terraform version manager", n = "tfenv", s = [{ description: "Install a specific version of Terraform", name: "install", args: [{ name: "version", description: "Possible Terraform Version", suggestions: ["latest", "min-required"] }] }, { description: "Switch to a version to use", name: "use", args: [{ name: "version", description: "Installed Terraform Version" }] }, { description: "Uninstall a specific version of Terraform", name: "uninstall", args: [{ name: "version", description: "Installed Terraform Version" }] }], r = [{ description: "View your current tfenv version", name: ["-v", "--version"] }, { description: "View commands", name: "--help" }], i = { description: e, name: n, subcommands: s, options: r };
export {
  i as default,
  e as description,
  n as name,
  r as options,
  s as subcommands
};
