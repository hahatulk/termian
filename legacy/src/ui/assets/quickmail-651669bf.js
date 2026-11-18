/*
 * Recovered Termius UI module: quickmail-651669bf.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Quickmail is a terminal-based solution to send mails", t = "quickmail", i = [{ description: "Initialise cli", name: "init", args: [{ template: ["filepaths"], isOptional: true }], options: [{ description: "Get help regarding init command", name: ["-h", "--help"] }] }, { description: "Clear the files stored locally", name: "clear", options: [{ description: "Get help regarding clear command", name: ["-h", "--help"] }, { description: "Clear entire cli storage including token", name: ["-j", "--justdoit"] }] }, { description: "Send mail", name: "send", options: [{ description: "Get help regarding send command", name: ["-h", "--help"] }, { description: "Receiver's email address", name: ["-r", "--receiver"], args: [{ name: "receiver" }] }, { description: "Subject of mail", name: ["-sub", "--subject"], args: [{ name: "Subject" }] }, { description: "Templates for mail body", name: ["-t", "--template"], args: [{ name: "Template", isOptional: true }] }, { description: "Body of the mail", name: ["-b", "--body"], args: [{ name: "Body", isOptional: true }] }, { description: "Path to attachment file", name: ["-a", "--attachment"], args: [{ name: "Attachment file", template: ["filepaths"], isOptional: true }] }, { description: "Skip confirmation, and send mail", name: ["-l", "--lessgo"] }] }, { description: "Manage templates", name: "template", subcommands: [{ description: "Add a new template", name: "add", options: [{ description: "Get help regarding template add subcommand", name: ["-h", "--help"] }, { description: "Set template id", name: "-n", args: [{ name: "Template id" }] }] }, { description: "List all templates stored", name: "listall", options: [{ description: "Get help regarding template listall subcommand", name: ["-h", "--help"] }] }, { description: "Edit any particular template", name: "edit", options: [{ description: "Get help regarding template edit subcommand", name: ["-h", "--help"] }, { description: "Edit template by id", name: "-n", args: [{ name: "Template id" }] }] }], options: [{ description: "Get help regarding template command", name: ["-h", "--help"] }] }], a = [{ description: "Output cli version", name: ["-v", "--version"] }, { description: "Output cli help", name: ["-h", "--help"] }], n = { flagsArePosixNoncompliant: true }, s = { description: e, name: t, subcommands: i, options: a, parserDirectives: n };
export {
  s as default,
  e as description,
  t as name,
  a as options,
  n as parserDirectives,
  i as subcommands
};
