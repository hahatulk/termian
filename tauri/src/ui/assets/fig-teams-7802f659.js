/*
 * Recovered Termius UI module: fig-teams-7802f659.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Fig for teams", a = "fig-teams@latest", s = [{ name: "teams", subcommands: [{ description: "Get all available teams for the current user", name: "ls", options: [{ description: "Output as JSON", name: "--json" }] }, { description: "Create a new team", name: "create", args: [{ name: "team_name" }] }, { description: "Get all users for a given team", name: "ls:user", options: [{ description: "Team to use", name: ["-t", "--team"], args: [{ name: "teamname" }] }, { description: "Output as JSON", name: "--json" }] }, { description: "Add a new user to a team", name: "add:user", args: [{ name: "email" }], options: [{ description: "Team to use", name: ["-t", "--team"], args: [{ name: "teamname" }] }] }, { description: "Remove a user from a team", name: "remove:user", args: [{ name: "email" }], options: [{ description: "Team to use", name: ["-t", "--team"], args: [{ name: "teamname" }] }] }] }], t = { description: e, name: a, subcommands: s };
export {
  t as default,
  e as description,
  a as name,
  s as subcommands
};
