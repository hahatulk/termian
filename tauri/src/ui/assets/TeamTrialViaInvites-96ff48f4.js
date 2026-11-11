/*
 * Recovered Termius UI module: TeamTrialViaInvites-96ff48f4.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as v, cN as R, bq as A, bj as c, wL as L, cO as N, z5 as W, z6 as V, xL as H, bt as e, pm as q, br as M, bP as $, xH as n, xd as G, z7 as J, bz as K, x$ as C, m4 as Q, bi as U, ew as f, y0 as X, bA as m, z8 as Y, o6 as h, z9 as w } from "./reconnectSaga-3b11f62e.js";
import { S as Z, a as _ } from "./ShareEntitiesSuccessScreen-c24a2654.js";
import { u as ee } from "./fadeIn-d66fb556.js";
import { I as te } from "./InviteSuccessScreen-1a17d0d0.js";
import "./RadioOptionsList-6f3efe10.js";
import "./applyTeamVaultTargetState-377d7128.js";
const le = v.forwardRef((k, E) => {
  const { closeDialog: l, teamTrialFunnelID: s, entryPointForAnalytics: p = "Invite People Button", ...P } = k, { classes: d } = re(), g = R(), o = A(), y = c(L), u = c(N), S = c(W), z = c(V), I = c(H), j = ee(P), [i, t] = v.useState("invite"), [x, B] = v.useState([{ email: "", role: "Editor" }]), F = async (a) => {
    t("loading");
    try {
      await Promise.allSettled(a.map((r) => f(g, w, { email: r.email, role: r.role }))), t("successInvites");
    } catch (r) {
      t("errorInvites"), h(r, "Inviting members to enterprise department failed");
    }
  }, b = async (a) => {
    if (B(a), I) {
      await F(a);
      return;
    }
    if (!u) {
      t("loading");
      try {
        await f(g, X, { createTeam: true, teamName: "My Team", force2fa: false, invites: a }), o(m("inviteButtonPressed", { numberOfInvitees: a.length })), S ? (t("sharing"), s && o(m("createTeamWizardViewOpened", { createTeamWizardStep: "Shared Data Picker", openedFrom: p, teamTrialFunnelID: s, trial: "yes" })), o(Y())) : (t("successInvites"), s && o(m("createTeamWizardViewOpened", { createTeamWizardStep: "Success", openedFrom: p, teamTrialFunnelID: s, trial: "yes" })));
      } catch (r) {
        t("errorInvites"), h(r, "Creating team trial and inviting members failed");
      }
    }
    if (y && u) try {
      t("loading");
      for await (const r of a) try {
        await f(g, w, { email: r.email, role: r.role });
      } catch (T) {
        if (T.status !== 400) throw T;
      }
      o(m("inviteButtonPressed", { numberOfInvitees: a.length })), t(z || !S ? "successInvites" : "sharing");
    } catch (r) {
      t("errorInvites"), h(r, "Invite team members failed");
    }
  }, O = () => {
    try {
      l();
    } catch (a) {
      t("error"), h(a, "Skip presenceEffect inviting failed");
    }
  }, D = y && u || I;
  return e(q.div, { style: j, ref: E, children: e("div", { className: d.dialog, children: M("div", { className: $(d.mainWrapper, { [d.invitePadding]: i === "invite" }), children: [e(n, { in: i === "invite", justifyContent: "center", alignItems: "center", children: e(G, { onSendInvites: b, onSendLater: O, leftButtonTitle: "Later", rightButtonTitle: "Continue", additionalRightButton: D ? e(J, {}) : void 0 }) }), e(n, { in: i === "loading", justifyContent: "center", alignItems: "center", children: e(K, { className: d.progressCircle, size: 205, thickness: 2 }) }), e(n, { in: i === "successInvites", justifyContent: "center", alignItems: "center", children: e(te, { onClose: () => l() }) }), e(n, { in: i === "errorInvites", justifyContent: "center", alignItems: "center", children: e(C, { onBack: () => t("invite"), onRetry: () => Q(x) ? b(x) : void 0, hasRetryBtn: true }) }), e(n, { in: i === "sharing", justifyContent: "center", alignItems: "center", children: e(Z, { teamTrialFunnelID: s, variant: "teamTrialFlow", onClose: () => l(), setContentType: t, entryPointForAnalytics: p }) }), e(n, { in: i === "successSharing", justifyContent: "center", alignItems: "center", children: e(_, { onClose: () => l() }) }), e(n, { in: i === "errorSharing", justifyContent: "center", alignItems: "center", children: e(C, { onBack: () => t("sharing") }) })] }) }) });
}), re = U()({ dialog: { "--text-color": "var(--dark-grey-1)", "--close-color": "var(--light-grey-1)", "--border-color": "var(--light-grey-5)", ".termius-dark-theme &": { "--text-color": "var(--white)", "--close-color": "var(--dark-grey-7)", "--border-color": "var(--dark-grey-2)" }, overflow: "hidden", width: "720px", height: "100%", borderRadius: "20px", boxShadow: "0px 5px 15px rgba(0, 0, 0, 0.15)", position: "relative", display: "flex", flexDirection: "column", "@media (max-height: 460px) and (max-width:730px)": { borderRadius: "0" } }, mainWrapper: { backgroundColor: "var(--foreground)", color: "var(--text-color)", width: "720px", height: "540px", "@media (max-height: 520px)": { height: "100vh" } }, invitePadding: { paddingTop: "90px" }, progressCircle: { color: "var(--accent)" } });
export {
  le as default
};
