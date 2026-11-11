/*
 * Recovered Termius UI module: dscacheutil-f4e9a12a.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Utility for managing the Directory Service cache", t = "dscacheutil", o = [{ description: "Query the Directory Service cache", name: "-q", args: [{ name: "category", description: "Category to query", suggestions: ["group", "host", "mount", "protocol", "rpc", "service", "user"] }], options: [{ description: "Attribute to query", name: "-a", args: [{ name: "attributeKey" }, { name: "attributeValue" }] }] }, { description: "Get an overview of the cache by default", name: "-cachedump", options: [{ description: "Get an overview of the cache by default", name: "-buckets" }, { description: "Dump detailed information about cache entries", name: "-entries", args: [{ name: "entry", suggestions: ["group", "host", "mount", "protocol", "rpc", "service", "user"] }] }] }], r = { description: e, name: t, subcommands: o };
export {
  r as default,
  e as description,
  t as name,
  o as subcommands
};
