/*
 * Recovered Termius UI module: do-release-upgrade-e70e7e23.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Upgrade Ubuntu to latest release", n = "do-release-upgrade", s = [{ description: "Show help message and exit", name: ["-h", "--help"] }, { description: "If using the latest supported release, upgrade to the development release", name: ["-d", "--devel-release"] }, { description: "Try upgrading to the latest release using the upgrader from Ubuntu-proposed", name: ["-p", "--proposed"] }, { description: "Run in a special upgrade mode", name: "-m", args: [{ name: "mode", suggestions: ["desktop", "server"] }] }, { description: "Run in a special upgrade mode", name: "--mode", args: [{ name: "mode", suggestions: ["desktop", "server"] }] }, { description: "Run the specified frontend", name: "-f", args: [{ name: "frontend" }] }, { description: "Run the specified frontend", name: "--frontend", args: [{ name: "frontend" }] }], t = { description: e, name: n, options: s };
export {
  t as default,
  e as description,
  n as name,
  s as options
};
