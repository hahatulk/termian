/*
 * Recovered Termius UI module: PortForwardingRulePresenter-4942c31a.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
class t {
  label(e) {
    return e.hostnames;
  }
  description() {
    return "";
  }
  icon() {
    return "fingerprint";
  }
}
const c = new t();
class o {
  label(e) {
    return !e.label && e.isValid ? e.rule : e.label;
  }
  placeholder() {
    return "Enter IP or Hostname...";
  }
  description(e) {
    return e.isValid ? e.rule : "";
  }
  icon(e) {
    const r = e.status === "connected";
    switch (e.pf_type) {
      case "Remote Rule":
        return r ? "connected_remote_pf" : "remote_pf";
      case "Dynamic Rule":
        return r ? "connected_dynamic_pf" : "dynamic_pf";
      default:
        return r ? "connected_local_pf" : "local_pf";
    }
  }
}
const s = new o();
export {
  c as k,
  s as p
};
