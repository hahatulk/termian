/*
 * Recovered Termius UI module: github-e8917e98.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Open a git repository in GitHub Desktop", n = "github", t = [{ description: "Clone a repository", name: "clone", args: [{ name: "url|slug", suggestions: [{ name: "https://github.com/" }, { name: "https://gitlab.com/" }, { name: "https://bitbucket.com/" }] }], options: [{ description: "The branch to checkout after cloning", name: ["--branch", "-b"], args: [{ name: "branch" }] }] }, { description: "Open a git repository in GitHub Desktop", name: "open", args: [{ name: "path", template: ["folders"], isOptional: true }] }, { description: "Show the help page for a command", name: "help", args: [{ name: "command", suggestions: ["clone", "open", "help"], isOptional: true }] }], o = [{ name: "path", template: ["folders"], isOptional: true }], s = [{ description: "Show the help page for a command", name: "--help", args: [{ name: "command", suggestions: ["clone", "open", "help"], isOptional: true }] }], a = { description: e, name: n, subcommands: t, args: o, options: s };
export {
  o as args,
  a as default,
  e as description,
  n as name,
  s as options,
  t as subcommands
};
