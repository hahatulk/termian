/*
 * Recovered Termius UI module: who-068b388f.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
const e = "Display who is logged in", t = "who", n = [], i = [{ name: "file", description: "By default, who gathers information from the file /var/run/utmpx; an alternative file may be specified", template: ["filepaths"], isOptional: true }], s = [{ description: "Same as -bdlprTtu", name: "-a" }, { description: "Time of last system boot", name: "-b" }, { description: "Print dead processes", name: "-d" }, { description: "Write column headings above the regular output", name: "-H" }, { description: "Print system login processes (unsupported)", name: "-l" }, { description: "Only print information about the current terminal", name: "-m" }, { description: "Print active processes spawned by launchd(8) (unsupported)", name: "-p" }, { description: "'Quick mode': List only names and number of users currently logged on", name: "-q" }, { description: "Print the current runlevel", name: "-r" }, { description: "List only the name, line and time fields (this is the default)", name: "-s" }, { description: "Print a character after the user name indicating the state of the terminal line: '+' writable, '-' not writable, '?' bad", name: "-T" }, { description: "Print last system clock change (unsupported)", name: "-t" }, { description: "Print the idle time for each user and the associated process ID", name: "-u" }], r = { optionsMustPrecedeArguments: true }, a = { description: e, name: t, subcommands: n, args: i, options: s, parserDirectives: r };
export {
  i as args,
  a as default,
  e as description,
  t as name,
  s as options,
  r as parserDirectives,
  n as subcommands
};
