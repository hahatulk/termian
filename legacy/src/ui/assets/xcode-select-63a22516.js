/*
 * Recovered Termius UI module: xcode-select-63a22516.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Active developer directory for Xcode tools", t = "xcode-select", o = [{ description: "Help message", name: ["-h", "--help"] }, { description: "Display path to active developer directory", name: ["-p", "--print-path"] }, { insertValue: "-s ", description: "Set path to active developer directory", name: ["-s", "--switch"], args: [{ name: "path", template: ["folders"] }] }, { description: "Install Xcode command line tools", name: "--install" }, { description: "Display version", name: ["-v", "--version"] }, { description: "Reset to the default CLT path", name: ["-r", "--reset"] }], s = { description: e, name: t, options: o };
export {
  s as default,
  e as description,
  t as name,
  o as options
};
