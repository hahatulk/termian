/*
 * Recovered Termius UI module: mkfifo-fb7c63d6.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Make FIFOs (first-in, first-out)", s = "mkfifo", t = [{ name: "FIFO", description: "FIFO(s) to create", isVariadic: true }], i = [{ description: "Set the file permission bits of newly-created fifos to mode, without respect to the current umask; the mode is specified as in chmod(1)", name: "-m", args: [{ name: "MODE", suggestions: ["0666", "0644", "0444"] }] }], o = { description: e, name: s, args: t, options: i };
export {
  t as args,
  o as default,
  e as description,
  s as name,
  i as options
};
