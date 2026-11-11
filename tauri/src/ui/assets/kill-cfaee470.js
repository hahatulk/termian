/*
 * Recovered Termius UI module: kill-cfaee470.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const s = "Terminate or signal a process", n = "kill", e = [{ name: "pid", isVariadic: true }], i = [{ description: "A symbolic signal name specifying the signal to be sent", name: "-s", args: [{ name: "signal_name" }] }, { description: "If no operand is given, list the signal names; otherwise, write the signal name corresponding to exit_status", name: "-l", args: [{ name: "exit_status", isOptional: true }] }], t = { description: s, name: n, args: e, options: i };
export {
  e as args,
  t as default,
  s as description,
  n as name,
  i as options
};
