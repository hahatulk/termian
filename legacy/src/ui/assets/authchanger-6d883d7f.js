/*
 * Recovered Termius UI module: authchanger-6d883d7f.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Authchanger is a utility included with Jamf Connect to help you manage the authorization database used by macOS to determine how the login process progresses", t = "authchanger", n = [{ description: "Show help for authchanger", name: ["-help", "-h"] }, { description: "Prints the version number", name: "-version" }, { description: "Prints the current list of authorization mechanisms", name: "-print" }, { description: "Does a dry run of the changes and prints out what would have happened", name: "-debug" }, { description: "Resets the login screen to the factory settings", name: "-reset" }, { description: "Sets up Jamf Connect Login to only demobilze accounts", name: "-demobilize" }, { description: "Adds the DEP Notify addition to the corresponding -AD, or -Setup argument", name: "-notify" }, { description: "Adds the DEP Notify addition to the corresponding -AD, or -Setup argument", name: "-setup" }, { description: "Enables Jamf Connect 2.0 login mechanisms", name: "-jamfconnect" }, { description: "Enables verbose logging", name: "-verbose" }, { description: "Mechs to be used before the actual UI is shown", name: "-prelogin" }, { description: "Mechs to be used between the login UI and actual authentication", name: "-preauth" }, { description: "Mechs to be used after system authentication", name: "-postauth" }, { description: "Allows the printout of any authorizationDB rule as well as setting of that rule to any custom mechanism(s)", name: "-customrule" }, { description: "Enables Azure authentication for the Network Preference Pane", name: "-sysprefs" }, { description: "Removes Azure authentication for the Network Preference Pane", name: "-sysprefsreset" }, { description: "Adds the mechanism to be used with the sudosaml binary", name: "-defaultjcright" }], o = { flagsArePosixNoncompliant: true }, i = { description: e, name: t, options: n, parserDirectives: o };
export {
  i as default,
  e as description,
  t as name,
  n as options,
  o as parserDirectives
};
