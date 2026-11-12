/*
 * Recovered Termius UI module: defaults-c93269d6.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Command line interface to a user's defaults", a = "defaults", n = [{ description: "Shows defaults", name: "read", args: [{ name: "domain", suggestions: [{ description: "Global domain", name: "-globalDomain" }, { insertValue: "-app '{cursor}'", description: "Application name", name: "-app" }] }, { name: "key" }] }, { description: "Writes key for domain", name: "write", args: [{ name: "domain", suggestions: [{ description: "Global domain", name: "-globalDomain" }, { insertValue: "-app '{cursor}'", description: "Application name", name: "-app" }] }, { name: "key" }, { name: "value" }] }, { description: "Deletes domain or key in domain", name: "delete", args: [{ name: "domain", suggestions: [{ description: "Global domain", name: "-globalDomain" }, { insertValue: "-app '{cursor}'", description: "Application name", name: "-app" }] }, { name: "key" }] }, { description: "Renames old_key to new_key", name: "rename", args: [{ name: "domain", suggestions: [{ description: "Global domain", name: "-globalDomain" }, { insertValue: "-app '{cursor}'", description: "Application name", name: "-app" }] }, { name: "old_key" }, { name: "new_key" }] }, { description: "Lists all entries containing word", name: "find", args: [{ name: "word", description: "The word to search for" }] }, { description: "Shows the type for the given domain, key", name: "read-type", args: [{ name: "domain", suggestions: [{ description: "Global domain", name: "-globalDomain" }, { insertValue: "-app '{cursor}'", description: "Application name", name: "-app" }] }, { name: "key" }] }], i = { description: e, name: a, subcommands: n };
export {
  i as default,
  e as description,
  a as name,
  n as subcommands
};
