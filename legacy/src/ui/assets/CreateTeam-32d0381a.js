/*
 * Recovered Termius UI module: CreateTeam-32d0381a.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { bq as S, cN as g, bj as r, BE as v, BF as y, gG as F, r as i, cl as w, x1 as B, ay as I, bA as a, bt as t, br as x, bE as O, bF as P, BG as D, bi as W, cv as k, f1 as C, f2 as z, aH as E } from "./reconnectSaga-3b11f62e.js";
import { L as A } from "./LinkButton-03e176fc.js";
import { S as N } from "./SettingsTabContent-ce30872f.js";
function G() {
  const { classes: s } = j(), e = S(), p = g(), o = r(v), l = r(y), c = r(F), m = i.useRef(w.v4()), n = m.current, u = () => {
    e(a("inviteScreenOpened", { funnelType: "Team" })), e(a("inviteButtonPressed", { numberOfInvitees: 0 })), e(a("teamTrialFunnelStarted", { openedFrom: "Preferences", teamTrialFunnelID: n, teamTrialPlan: "Team" })), e(a("createTeamWizardViewOpened", { createTeamWizardStep: "Invite Members", openedFrom: "Invite People Button", teamTrialFunnelID: n, trial: "yes" }));
  }, d = async () => {
    u(), await C(p, { dialog: "TeamTrialViaInvites", target: z(), props: { teamTrialFunnelID: n }, options: { closeOnBackdropClick: false } }).catch(E);
  }, T = i.useCallback(() => {
    B({ username: c, plan: "team" });
  }, [c]), f = async (h) => {
    h.preventDefault(), l ? await d() : T();
  };
  i.useEffect(() => {
    o && (I.setDeprecatedWasTeamTrialInPreferencesOpened(), e(a("createTeamWizardViewOpened", { createTeamWizardStep: "Description Invite People", openedFrom: "Preferences", teamTrialFunnelID: m.current, trial: "yes" })));
  }, [e, o]);
  const b = l ? k : A;
  return t(N, { children: x(O, { offsetTop: 150, children: [t(P, { title: "Invite your team", text: `Manage infrastructure together in shared team vaults.
            Keep your teammates on the same page and boost their productivity.`, classNameTitle: s.title, icon: t(D, { height: "25px", width: "25px", role: "img", "aria-label": "People with plus filled" }) }), t("form", { onSubmit: f, children: t("div", { className: s.controls, children: t(b, { className: s.inviteButton, size: "medium", color: "accent", autoFocus: true, type: "submit", children: "Invite members" }) }) })] }) });
}
const j = W()({ title: { fontSize: "24px", fontWeight: 900, lineHeight: "150%" }, controls: { display: "block", marginTop: "40px" }, inviteButton: { "&:focus-visible": { background: "var(--buttonColor)" } } });
export {
  G as default
};
