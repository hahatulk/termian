/*
 * Recovered Termius UI module: ln-70e62b6f.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Create (default hard) symbolic links to files", n = "ln", i = [{ name: "source_file", template: ["filepaths", "folders"] }, { name: "link_name or link_dirname", isOptional: true }], a = [{ description: "Create a symbolic link", name: "-s", args: [{ name: "source_file", template: ["filepaths", "folders"] }, { name: "link_name or link_dirname", isOptional: true }] }, { description: "Verbose", name: "-v" }, { description: "If link name already exists replace it", name: "-F", args: [{ name: "source_file", template: ["filepaths", "folders"] }, { name: "link_name or link_dirname", isOptional: true }] }, { description: "Don't follow symbolic links", name: "-h" }, { description: "If link name already exists unlink the old one before creating the new one", name: "-f", args: [{ name: "source_file", template: ["filepaths", "folders"] }, { name: "link_name or link_dirname", isOptional: true }] }, { description: "Prompt if proposed link already exists", name: "-i", args: [{ name: "source_file", template: ["filepaths", "folders"] }, { name: "link_name or link_dirname", isOptional: true }] }, { description: "Same as -h don't follow symbolic links", name: "-n" }], l = { description: e, name: n, args: i, options: a };
export {
  i as args,
  l as default,
  e as description,
  n as name,
  a as options
};
