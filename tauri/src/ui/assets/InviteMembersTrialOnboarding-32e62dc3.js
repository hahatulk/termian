/*
 * Recovered Termius UI module: InviteMembersTrialOnboarding-32e62dc3.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as i, cN as b, cl as T, bq as w, bj as j, x_ as k, bt as e, pm as D, br as d, xH as o, xd as R, bz as F, x$ as N, bi as B, ew as A, y0 as O, bA as z, y1 as E, o6 as m, y2 as P } from "./reconnectSaga-3b11f62e.js";
import { C as q } from "./CloseAndBackDialogSection-7ee44c0d.js";
import { u as H } from "./fadeIn-d66fb556.js";
import { I as M } from "./InviteSuccessScreen-1a17d0d0.js";
const J = i.forwardRef((p, u) => {
  const { closeDialog: r, ...h } = p, { classes: t } = W(), g = b(), v = i.useRef(T.v4()), c = w(), x = H(h), [a, n] = i.useState("invite"), [f, y] = i.useState([{ email: "", role: "Editor" }]), S = j(k), l = async (s) => {
    n("loading"), y(s);
    try {
      await A(g, O, { createTeam: true, teamName: "My Team", force2fa: false, invites: s }), c(z("teamTrialFunnelStarted", { openedFrom: "Trial Onboarding", teamTrialFunnelID: v.current, teamTrialPlan: "Team" })), c(E({ onboardingAction: "Invite", success: "yes", remainingSteps: S ? 0 : 1 })), n("success");
    } catch (C) {
      n("error"), m(C, "Invite members failed");
    }
  }, I = () => {
    try {
      c(P.request()), r();
    } catch (s) {
      n("error"), m(s, "Skip invite step failed");
    }
  };
  return e(D.div, { style: x, ref: u, children: d("div", { className: t.dialog, children: [e(q, { onClose: r, onBack: r, className: t.titleCenter }), d("div", { className: t.mainWrapper, children: [e(o, { in: a === "invite", justifyContent: "center", alignItems: "center", children: e(R, { onSendInvites: l, onSendLater: I, leftButtonTitle: "I don't have a team", rightButtonTitle: "Invite" }) }), e(o, { in: a === "loading", justifyContent: "center", alignItems: "center", children: e("div", { className: t.spinnerCenter, children: e(F, { className: t.progressCircle, size: 205, thickness: 2 }) }) }), e(o, { in: a === "success", justifyContent: "center", alignItems: "center", children: e(M, { onClose: () => r() }) }), e(o, { in: a === "error", justifyContent: "center", alignItems: "center", children: e(N, { onBack: () => n("invite"), onRetry: () => l(f), hasRetryBtn: true }) })] })] }) });
}), W = B()({ dialog: { overflow: "hidden", width: "720px", height: "100%", borderRadius: "20px", boxShadow: "0px 5px 15px rgba(0, 0, 0, 0.15)", position: "relative", display: "flex", flexDirection: "column", "@media (max-height: 520px)": { height: "100vh" }, "@media (max-height: 520px) and (max-width:730px)": { borderRadius: "0", width: "100vw" } }, mainWrapper: { backgroundColor: "var(--foreground)", color: "var(--text-color)", width: "720px", height: "540px" }, titleCenter: { justifyContent: "center", zIndex: 1 }, spinnerCenter: { width: "100%", height: "540px", display: "flex", justifyContent: "center", alignItems: "center" }, progressCircle: { color: "var(--accent)" } });
export {
  J as default
};
