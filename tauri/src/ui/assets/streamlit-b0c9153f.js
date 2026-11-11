/*
 * Recovered Termius UI module: streamlit-b0c9153f.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Streamlit", n = "streamlit", t = [{ description: "Run a Python script, piping stderr to Streamlit", name: "run", args: [{ name: "file", description: "The Python script to run" }] }], s = [{ description: "Set the log level", name: "--log_level", args: [{ suggestions: [{ name: "error" }, { name: "warning" }, { name: "info" }, { name: "debug" }] }] }, { description: "Show a help message and exit", name: "--help" }, { description: "Show the version and exit", name: "--version" }], i = { description: e, name: n, subcommands: t, options: s };
export {
  i as default,
  e as description,
  n as name,
  s as options,
  t as subcommands
};
