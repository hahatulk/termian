/*
 * Recovered Termius UI module: sudo-8cbb3a25.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Execute a command as the superuser or another user", s = "sudo", n = [{ name: "command", description: "Command to run with elevated permissions", isCommand: true }], o = [{ description: "Run command as the specified group name or ID", name: ["-g", "--group"], args: [{ name: "group", description: "Group name or ID" }] }, { description: "Display help message and exit", name: ["-h", "--help"] }, { description: "Run command as specified user name or ID", name: ["-u", "--user"], args: [{ name: "user", description: "User name or ID" }] }], a = { description: e, name: s, args: n, options: o };
export {
  n as args,
  a as default,
  e as description,
  s as name,
  o as options
};
