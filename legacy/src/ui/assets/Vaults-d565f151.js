/*
 * Recovered Termius UI module: Vaults-d565f151.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as O, hc as it, wm as ot, CT as ue, bC as De, CU as Oe, CV as F, c_ as me, CW as lt, CX as ct, bi as B, bj as w, CY as ne, bq as $, g4 as dt, bA as L, bt as n, bs as f, br as p, bu as C, hW as ut, xy as mt, cv as Y, lG as pt, zC as ht, gO as Z, bx as W, xB as gt, Aw as pe, pE as Me, CZ as ft, d3 as vt, ac as bt, by as Ne, d2 as yt, C_ as J, bO as _, cK as Te, bk as Ve, C$ as xt, D0 as wt, fv as Re, bw as _e, D1 as Le, D2 as oe, e0 as ve, dS as he, dT as P, Am as Ct, dG as Et, cX as St, b$ as kt, c0 as At, c1 as It, c2 as Dt, Bn as Ot, xz as Pe, D3 as Be, D4 as Fe, jV as Ue, k1 as Mt, yl as Nt, D5 as Tt, eb as Vt, D6 as Rt, f2 as $e, dF as _t, eF as Lt, dk as Pt, dn as Bt, dl as Ft, en as Ut, ep as $t, eq as Wt, et as jt, bX as We, gG as ge, Av as fe, D7 as Ht, D8 as le, g0 as be, D9 as U, Da as zt, bz as qt, e7 as Yt, Db as Kt, cR as Gt, cN as je, jz as Xt, cU as Zt, f1 as Qt, aH as Jt, ew as se, Dc as en, a6 as ce, Dd as tn, De as nn, aY as rn, xK as ie, bn as an, ft as sn, qo as on, a1 as G, Df as ln, Dg as He, Dh as te, Di as j, Dj as cn, e3 as dn, Dk as un, Dl as mn, E as pn, ca as hn, M as gn, Dm as fn, Dn as ye, eI as vn, Do as bn, xN as yn, xO as xn, Dp as wn, Dq as Cn } from "./reconnectSaga-3b11f62e.js";
import { P as En } from "./PageWrapper-a7d0cf6b.js";
import { M as Sn, a as kn, b as An, i as In } from "./vaultStatsIconsMap-a07c2efa.js";
import { U as ze } from "./UserListItem-e815dc7d.js";
import { M as Dn } from "./crownIcon.react-c8ab03e3.js";
import { S as On } from "./SelectMemberRole-bca8115e.js";
import { D as Mn } from "./Divider-8ca798ff.js";
import { M as Nn } from "./checkIconGradient.react-d8f1a390.js";
import { C as Tn } from "./ClickAwayListener-fa29c305.js";
const Vn = (e) => e.vaultsManagement, Rn = (e, t) => O.createElement("svg", { xmlns: "http://www.w3.org/2000/svg", width: 14, height: 15, viewBox: "0 0 14 15", fill: "none", ref: t, ...e }, O.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M7 14.5C10.866 14.5 14 11.366 14 7.5C14 3.63401 10.866 0.5 7 0.5C3.13401 0.5 0 3.63401 0 7.5C0 11.366 3.13401 14.5 7 14.5ZM6.125 4C6.125 3.51675 6.51675 3.125 7 3.125C7.48325 3.125 7.875 3.51675 7.875 4V7.5C7.875 7.98325 7.48325 8.375 7 8.375C6.51675 8.375 6.125 7.98325 6.125 7.5V4ZM6.125 10.1523C6.125 9.65398 6.52898 9.25 7.02731 9.25C7.52564 9.25 7.92962 9.65398 7.92962 10.1523V10.2204C7.92962 10.7187 7.52564 11.1227 7.02731 11.1227C6.52898 11.1227 6.125 10.7187 6.125 10.2204V10.1523Z", fill: "currentColor" })), _n = O.forwardRef(Rn), Ln = O.memo(_n), Pn = it([ot, ue, De], (e, t, r) => {
  const s = (r == null ? void 0 : r.id) ?? Oe;
  if ((t == null ? void 0 : t.owner_id) == null || (t == null ? void 0 : t.owner) == null) return [];
  const a = e.filter((i) => i.is_access_granted).map((i) => ({ email: i.email, full_name: i.full_name, user_id: i.user_id, role: i.role, vault_id: s }));
  return a.unshift({ role: "Editor", user_id: t.owner_id, email: t.owner, full_name: t.owner_name, vault_id: s }), a;
}), qe = (e, t) => {
  if (F(t)) return;
  const { entities: { hosts: s, snippets: a, portForwardingRules: i, sshKeys: o, visibleIdentities: d } } = e, l = (u) => u.filter((c) => me(t) ? lt(c) : ct(c, t)).length;
  return { hosts: l(s), snippets: l(a), portForwardingRules: l(i), sshKeys: l(o), visibleIdentities: l(d) };
}, Bn = ({ vault: e, onClose: t, onDelete: r, progress: s }) => {
  const { classes: a } = Ye(), i = w((u) => qe(u, e)), o = w(ne).find((u) => u.id === e.id), d = i && Object.values(i).some((u) => u > 0), l = $();
  return dt(() => {
    l(L("deleteVaultScreenOpened", { hasVaultDataAccess: o ? "no" : "yes", vaultID: e.id, numberOfHosts: (i == null ? void 0 : i.hosts) || 0, numberOfSshKeys: (i == null ? void 0 : i.sshKeys) || 0, numberOfSnippets: (i == null ? void 0 : i.snippets) || 0, numberOfIdentities: (i == null ? void 0 : i.visibleIdentities) || 0 }));
  }), n(f, { paddingX: "15px", children: n(f, { alignItems: "center", width: "100%", className: a.card, children: p(f, { width: "100%", gap: "20px", children: [p(f, { gap: "10px", children: [n(C, { color: "primary", children: "You are going to delete this vault:" }), p(f, { width: "100%", direction: "row", justifyContent: "space-between", className: a.statItem, children: [p(f, { direction: "row", alignItems: "center", gap: "5px", overflow: "hidden", children: [n(f, { alignItems: "center", justifyContent: "center", width: "24px", height: "24px", flexShrink: 0, children: n(ut, {}) }), n(C, { variant: "body1", color: "primary", ellipsis: true, children: e.name ?? "" })] }), p(f, { direction: "row", gap: "5px", alignItems: "center", className: a.membersCount, children: [n(mt, {}), n(C, { variant: "body2", color: "secondary", children: e.members.length })] })] })] }), d && p(f, { gap: "10px", children: [n(C, { color: "primary", children: "And all data inside:" }), n(f, { gap: "5px", component: "ul", children: Object.entries(i).map(([u, c]) => {
    const { icon: m, label: h } = Fn[u];
    return n(Un, { icon: m, count: c, name: c === 1 ? h.single : h.plural }, u);
  }) })] }), o && p(f, { gap: "10px", direction: "row", alignItems: "center", children: [n(Ln, { style: { color: "var(--secondary)" } }), n(C, { color: "secondary", variant: "body1", children: "All data in this vault will be deleted" })] }), p(f, { gap: "10px", width: "100%", children: [r && n(Y, { color: "error", onClick: () => {
    l(L("deleteVaultButtonPressed", { vaultID: e.id, hasVaultDataAccess: o ? "no" : "yes" })), r();
  }, fullWidth: true, progress: s, children: "Delete" }), n(Y, { disabled: s, variant: "text", onClick: t, fullWidth: true, children: "Cancel" })] })] }) }) });
}, Fn = { hosts: { icon: n(Sn, { color: "var(--main)" }), label: { single: "host", plural: "hosts" } }, snippets: { icon: n(kn, { color: "var(--main)" }), label: { single: "snippet", plural: "snippets" } }, portForwardingRules: { icon: n(pt, { width: "14px", color: "var(--main)" }), label: { single: "port forwarding", plural: "port forwardings" } }, sshKeys: { icon: n(ht, { color: "var(--main)" }), label: { single: "key", plural: "keys" } }, visibleIdentities: { icon: n(An, { color: "var(--main)" }), label: { single: "identity", plural: "identities" } } }, Un = ({ icon: e, count: t, name: r }) => {
  const { classes: s } = Ye();
  return t > 0 ? p(f, { component: "li", direction: "row", alignItems: "center", gap: "5px", className: s.statItem, children: [n(f, { alignItems: "center", justifyContent: "center", width: "24px", height: "24px", children: e }), p(C, { variant: "body1", color: "primary", children: [t, " ", r] }), n("br", {})] }) : null;
}, Ye = B()({ card: { padding: "20px 20px 20px 20px", backgroundColor: "var(--card-a)", borderRadius: "15px" }, statItem: { padding: "10px", borderRadius: "10px", backgroundColor: "var(--background)" }, membersCount: { color: "var(--secondary)" } }), Ke = ({ variant: e }) => {
  const { classes: t } = Q();
  return e === "hidden" ? null : n(f, { gap: "10px", children: n(f, { className: t.banner, direction: "row", justifyContent: "center", alignItems: "center", gap: "5px", children: zn[e] }) });
}, $n = () => {
  const { classes: e } = Q(), t = w(Z);
  return p(C, { variant: "body2", color: "secondary", children: [n(W, { size: 12, icon: n(gt, { className: e.teamInlineIcon }), className: e.iconContainer }), "New members will be automatically added to this vault and will see all data inside.", " ", t && p(f, { className: e.externalLink, onClick: pe, direction: "row", alignContent: "center", gap: "2px", children: [n(C, { color: "accent", variant: "body2", display: "inline", children: "Invite members" }), n(Me, { className: e.externalLinkIcon })] })] });
}, Wn = () => {
  const { classes: e, cx: t } = Q();
  return p(C, { variant: "body2", color: "secondary", display: "inline", children: [n(W, { size: 12, icon: n(ft, { className: t(e.icon, e.personalIconInline) }), className: e.iconContainer }), "Only you can access data in this vault. Personal credentials can be used to connect to entities in other vaults."] });
}, jn = () => {
  const { classes: e, cx: t } = Q();
  return p(C, { variant: "body2", color: "secondary", display: "inline", children: [n(W, { size: 12, icon: n(vt, { className: t(e.icon, e.noAccessIconInline) }), className: e.iconContainer }), "Contact your team admin to request access."] });
}, Hn = () => {
  var r;
  const { classes: e } = Q(), t = (r = w(ue)) == null ? void 0 : r.owner;
  return p(C, { variant: "body2", color: "secondary", className: e.memberBannerRoot, children: [n(f, { display: "inline-block", className: e.ownerEmail, children: t || "Team owner" }), " ", "has permissions to manage this vault"] });
}, zn = { team: n($n, {}), personal: n(Wn, {}), noAccess: n(jn, {}), withOwnerAccess: n(Hn, {}) }, Q = B()({ banner: { color: "var(--secondary)", padding: "10px", borderRadius: "10px", backgroundColor: "var(--background)" }, iconContainer: { display: "inline", marginRight: "5px" }, icon: { color: "var(--secondary)", width: "12px", height: "12px" }, noAccessIconInline: { marginBottom: "-2px" }, personalIconInline: { marginBottom: "-2px", marginLeft: "-2px" }, teamInlineIcon: { color: "var(--secondary)", marginBottom: "-3px" }, externalLinkIcon: { width: "12px", height: "12px" }, externalLink: { cursor: "pointer", display: "inline-flex", alignItems: "center", color: "var(--accent)" }, memberBannerRoot: { overflow: "hidden" }, ownerEmail: { maxWidth: "100%", whiteSpace: "nowrap", textOverflow: "ellipsis", overflow: "hidden", lineHeight: "inherit", verticalAlign: "bottom" } });
var re = { exports: {} }, Ge = { À: "A", Á: "A", Â: "A", Ã: "A", Ä: "A", Å: "A", Ấ: "A", Ắ: "A", Ẳ: "A", Ẵ: "A", Ặ: "A", Æ: "AE", Ầ: "A", Ằ: "A", Ȃ: "A", Ả: "A", Ạ: "A", Ẩ: "A", Ẫ: "A", Ậ: "A", Ç: "C", Ḉ: "C", È: "E", É: "E", Ê: "E", Ë: "E", Ế: "E", Ḗ: "E", Ề: "E", Ḕ: "E", Ḝ: "E", Ȇ: "E", Ẻ: "E", Ẽ: "E", Ẹ: "E", Ể: "E", Ễ: "E", Ệ: "E", Ì: "I", Í: "I", Î: "I", Ï: "I", Ḯ: "I", Ȋ: "I", Ỉ: "I", Ị: "I", Ð: "D", Ñ: "N", Ò: "O", Ó: "O", Ô: "O", Õ: "O", Ö: "O", Ø: "O", Ố: "O", Ṍ: "O", Ṓ: "O", Ȏ: "O", Ỏ: "O", Ọ: "O", Ổ: "O", Ỗ: "O", Ộ: "O", Ờ: "O", Ở: "O", Ỡ: "O", Ớ: "O", Ợ: "O", Ù: "U", Ú: "U", Û: "U", Ü: "U", Ủ: "U", Ụ: "U", Ử: "U", Ữ: "U", Ự: "U", Ý: "Y", à: "a", á: "a", â: "a", ã: "a", ä: "a", å: "a", ấ: "a", ắ: "a", ẳ: "a", ẵ: "a", ặ: "a", æ: "ae", ầ: "a", ằ: "a", ȃ: "a", ả: "a", ạ: "a", ẩ: "a", ẫ: "a", ậ: "a", ç: "c", ḉ: "c", è: "e", é: "e", ê: "e", ë: "e", ế: "e", ḗ: "e", ề: "e", ḕ: "e", ḝ: "e", ȇ: "e", ẻ: "e", ẽ: "e", ẹ: "e", ể: "e", ễ: "e", ệ: "e", ì: "i", í: "i", î: "i", ï: "i", ḯ: "i", ȋ: "i", ỉ: "i", ị: "i", ð: "d", ñ: "n", ò: "o", ó: "o", ô: "o", õ: "o", ö: "o", ø: "o", ố: "o", ṍ: "o", ṓ: "o", ȏ: "o", ỏ: "o", ọ: "o", ổ: "o", ỗ: "o", ộ: "o", ờ: "o", ở: "o", ỡ: "o", ớ: "o", ợ: "o", ù: "u", ú: "u", û: "u", ü: "u", ủ: "u", ụ: "u", ử: "u", ữ: "u", ự: "u", ý: "y", ÿ: "y", Ā: "A", ā: "a", Ă: "A", ă: "a", Ą: "A", ą: "a", Ć: "C", ć: "c", Ĉ: "C", ĉ: "c", Ċ: "C", ċ: "c", Č: "C", č: "c", C̆: "C", c̆: "c", Ď: "D", ď: "d", Đ: "D", đ: "d", Ē: "E", ē: "e", Ĕ: "E", ĕ: "e", Ė: "E", ė: "e", Ę: "E", ę: "e", Ě: "E", ě: "e", Ĝ: "G", Ǵ: "G", ĝ: "g", ǵ: "g", Ğ: "G", ğ: "g", Ġ: "G", ġ: "g", Ģ: "G", ģ: "g", Ĥ: "H", ĥ: "h", Ħ: "H", ħ: "h", Ḫ: "H", ḫ: "h", Ĩ: "I", ĩ: "i", Ī: "I", ī: "i", Ĭ: "I", ĭ: "i", Į: "I", į: "i", İ: "I", ı: "i", Ĳ: "IJ", ĳ: "ij", Ĵ: "J", ĵ: "j", Ķ: "K", ķ: "k", Ḱ: "K", ḱ: "k", K̆: "K", k̆: "k", Ĺ: "L", ĺ: "l", Ļ: "L", ļ: "l", Ľ: "L", ľ: "l", Ŀ: "L", ŀ: "l", Ł: "l", ł: "l", Ḿ: "M", ḿ: "m", M̆: "M", m̆: "m", Ń: "N", ń: "n", Ņ: "N", ņ: "n", Ň: "N", ň: "n", ŉ: "n", N̆: "N", n̆: "n", Ō: "O", ō: "o", Ŏ: "O", ŏ: "o", Ő: "O", ő: "o", Œ: "OE", œ: "oe", P̆: "P", p̆: "p", Ŕ: "R", ŕ: "r", Ŗ: "R", ŗ: "r", Ř: "R", ř: "r", R̆: "R", r̆: "r", Ȓ: "R", ȓ: "r", Ś: "S", ś: "s", Ŝ: "S", ŝ: "s", Ş: "S", Ș: "S", ș: "s", ş: "s", Š: "S", š: "s", Ţ: "T", ţ: "t", ț: "t", Ț: "T", Ť: "T", ť: "t", Ŧ: "T", ŧ: "t", T̆: "T", t̆: "t", Ũ: "U", ũ: "u", Ū: "U", ū: "u", Ŭ: "U", ŭ: "u", Ů: "U", ů: "u", Ű: "U", ű: "u", Ų: "U", ų: "u", Ȗ: "U", ȗ: "u", V̆: "V", v̆: "v", Ŵ: "W", ŵ: "w", Ẃ: "W", ẃ: "w", X̆: "X", x̆: "x", Ŷ: "Y", ŷ: "y", Ÿ: "Y", Y̆: "Y", y̆: "y", Ź: "Z", ź: "z", Ż: "Z", ż: "z", Ž: "Z", ž: "z", ſ: "s", ƒ: "f", Ơ: "O", ơ: "o", Ư: "U", ư: "u", Ǎ: "A", ǎ: "a", Ǐ: "I", ǐ: "i", Ǒ: "O", ǒ: "o", Ǔ: "U", ǔ: "u", Ǖ: "U", ǖ: "u", Ǘ: "U", ǘ: "u", Ǚ: "U", ǚ: "u", Ǜ: "U", ǜ: "u", Ứ: "U", ứ: "u", Ṹ: "U", ṹ: "u", Ǻ: "A", ǻ: "a", Ǽ: "AE", ǽ: "ae", Ǿ: "O", ǿ: "o", Þ: "TH", þ: "th", Ṕ: "P", ṕ: "p", Ṥ: "S", ṥ: "s", X́: "X", x́: "x", Ѓ: "Г", ѓ: "г", Ќ: "К", ќ: "к", A̋: "A", a̋: "a", E̋: "E", e̋: "e", I̋: "I", i̋: "i", Ǹ: "N", ǹ: "n", Ồ: "O", ồ: "o", Ṑ: "O", ṑ: "o", Ừ: "U", ừ: "u", Ẁ: "W", ẁ: "w", Ỳ: "Y", ỳ: "y", Ȁ: "A", ȁ: "a", Ȅ: "E", ȅ: "e", Ȉ: "I", ȉ: "i", Ȍ: "O", ȍ: "o", Ȑ: "R", ȑ: "r", Ȕ: "U", ȕ: "u", B̌: "B", b̌: "b", Č̣: "C", č̣: "c", Ê̌: "E", ê̌: "e", F̌: "F", f̌: "f", Ǧ: "G", ǧ: "g", Ȟ: "H", ȟ: "h", J̌: "J", ǰ: "j", Ǩ: "K", ǩ: "k", M̌: "M", m̌: "m", P̌: "P", p̌: "p", Q̌: "Q", q̌: "q", Ř̩: "R", ř̩: "r", Ṧ: "S", ṧ: "s", V̌: "V", v̌: "v", W̌: "W", w̌: "w", X̌: "X", x̌: "x", Y̌: "Y", y̌: "y", A̧: "A", a̧: "a", B̧: "B", b̧: "b", Ḑ: "D", ḑ: "d", Ȩ: "E", ȩ: "e", Ɛ̧: "E", ɛ̧: "e", Ḩ: "H", ḩ: "h", I̧: "I", i̧: "i", Ɨ̧: "I", ɨ̧: "i", M̧: "M", m̧: "m", O̧: "O", o̧: "o", Q̧: "Q", q̧: "q", U̧: "U", u̧: "u", X̧: "X", x̧: "x", Z̧: "Z", z̧: "z", й: "и", Й: "И", ё: "е", Ё: "Е" }, Xe = Object.keys(Ge).join("|"), qn = new RegExp(Xe, "g"), Yn = new RegExp(Xe, "");
function Kn(e) {
  return Ge[e];
}
var Ze = function(e) {
  return e.replace(qn, Kn);
}, Gn = function(e) {
  return !!e.match(Yn);
};
re.exports = Ze;
re.exports.has = Gn;
re.exports.remove = Ze;
var Xn = re.exports;
const Zn = bt(Xn);
/**
* @name match-sorter
* @license MIT license.
* @copyright (c) 2020 Kent C. Dodds
* @author Kent C. Dodds <me@kentcdodds.com> (https://kentcdodds.com)
*/
const R = { CASE_SENSITIVE_EQUAL: 7, EQUAL: 6, STARTS_WITH: 5, WORD_STARTS_WITH: 4, CONTAINS: 3, ACRONYM: 2, MATCHES: 1, NO_MATCH: 0 }, Qn = (e, t) => String(e.rankedValue).localeCompare(String(t.rankedValue));
function H(e, t, r) {
  r === void 0 && (r = {});
  const { keys: s, threshold: a = R.MATCHES, baseSort: i = Qn, sorter: o = (u) => u.sort((c, m) => nr(c, m, i)) } = r, d = e.reduce(l, []);
  return o(d).map((u) => {
    let { item: c } = u;
    return c;
  });
  function l(u, c, m) {
    const h = Jn(c, s, t, r), { rank: S, keyThreshold: v = a } = h;
    return S >= v && u.push({ ...h, item: c, index: m }), u;
  }
}
H.rankings = R;
function Jn(e, t, r, s) {
  if (!t) {
    const i = e;
    return { rankedValue: i, rank: xe(i, r, s), keyIndex: -1, keyThreshold: s.threshold };
  }
  return sr(e, t).reduce((i, o, d) => {
    let { rank: l, rankedValue: u, keyIndex: c, keyThreshold: m } = i, { itemValue: h, attributes: S } = o, v = xe(h, r, s), x = u;
    const { minRanking: b, maxRanking: I, threshold: N } = S;
    return v < b && v >= R.MATCHES ? v = b : v > I && (v = I), v > l && (l = v, c = d, m = N, x = h), { rankedValue: x, rank: l, keyIndex: c, keyThreshold: m };
  }, { rankedValue: e, rank: R.NO_MATCH, keyIndex: -1, keyThreshold: s.threshold });
}
function xe(e, t, r) {
  return e = we(e, r), t = we(t, r), t.length > e.length ? R.NO_MATCH : e === t ? R.CASE_SENSITIVE_EQUAL : (e = e.toLowerCase(), t = t.toLowerCase(), e === t ? R.EQUAL : e.startsWith(t) ? R.STARTS_WITH : e.includes(` ${t}`) ? R.WORD_STARTS_WITH : e.includes(t) ? R.CONTAINS : t.length === 1 ? R.NO_MATCH : er(e).includes(t) ? R.ACRONYM : tr(e, t));
}
function er(e) {
  let t = "";
  return e.split(" ").forEach((s) => {
    s.split("-").forEach((i) => {
      t += i.substr(0, 1);
    });
  }), t;
}
function tr(e, t) {
  let r = 0, s = 0;
  function a(l, u, c) {
    for (let m = c, h = u.length; m < h; m++) if (u[m] === l) return r += 1, m + 1;
    return -1;
  }
  function i(l) {
    const u = 1 / l, c = r / t.length;
    return R.MATCHES + c * u;
  }
  const o = a(t[0], e, 0);
  if (o < 0) return R.NO_MATCH;
  s = o;
  for (let l = 1, u = t.length; l < u; l++) {
    const c = t[l];
    if (s = a(c, e, s), !(s > -1)) return R.NO_MATCH;
  }
  const d = s - o;
  return i(d);
}
function nr(e, t, r) {
  const { rank: i, keyIndex: o } = e, { rank: d, keyIndex: l } = t;
  return i === d ? o === l ? r(e, t) : o < l ? -1 : 1 : i > d ? -1 : 1;
}
function we(e, t) {
  let { keepDiacritics: r } = t;
  return e = `${e}`, r || (e = Zn(e)), e;
}
function rr(e, t) {
  typeof t == "object" && (t = t.key);
  let r;
  if (typeof t == "function") r = t(e);
  else if (e == null) r = null;
  else if (Object.hasOwnProperty.call(e, t)) r = e[t];
  else {
    if (t.includes(".")) return ar(t, e);
    r = null;
  }
  return r == null ? [] : Array.isArray(r) ? r : [String(r)];
}
function ar(e, t) {
  const r = e.split(".");
  let s = [t];
  for (let a = 0, i = r.length; a < i; a++) {
    const o = r[a];
    let d = [];
    for (let l = 0, u = s.length; l < u; l++) {
      const c = s[l];
      if (c != null) if (Object.hasOwnProperty.call(c, o)) {
        const m = c[o];
        m != null && d.push(m);
      } else o === "*" && (d = d.concat(c));
    }
    s = d;
  }
  return Array.isArray(s[0]) ? [].concat(...s) : s;
}
function sr(e, t) {
  const r = [];
  for (let s = 0, a = t.length; s < a; s++) {
    const i = t[s], o = ir(i), d = rr(e, i);
    for (let l = 0, u = d.length; l < u; l++) r.push({ itemValue: d[l], attributes: o });
  }
  return r;
}
const Ce = { maxRanking: 1 / 0, minRanking: -1 / 0 };
function ir(e) {
  return typeof e == "string" ? Ce : { ...Ce, ...e };
}
const de = ({ vault: e, membersCount: t, onClick: r, variant: s = "default", selected: a = false }) => {
  const { classes: i, cx: o } = Qe(), d = F(e), l = w((u) => qe(u, e));
  return p(f, { direction: "row", alignItems: "center", justifyContent: "space-between", className: o(i.root, { [i.selected]: a, [i.compact]: s === "compact" }), onClick: (u) => {
    u.stopPropagation(), r();
  }, children: [p("section", { className: i.leftSection, children: [n(W, { size: 24, icon: n(Ne, { width: "16px", color: "var(--main)" }) }), d && !e.name ? n(C, { variant: "body1", color: "secondary", ellipsis: true, children: "Enter vault name, e.g. Production..." }) : n(C, { title: e.name, variant: "body1", color: "primary", ellipsis: true, children: e.name })] }), l && !d && p("section", { className: i.rightSection, children: [s === "default" && n(or, { stats: l }), n(yt, { vault: e, membersCount: t })] })] });
}, or = ({ stats: e }) => {
  const { classes: t } = Qe();
  return e && Object.values(e).some((s) => s > 0) ? p(f, { className: t.statList, direction: "row", gap: "10px", alignItems: "center", children: [Object.entries(e).map(([s, a]) => a > 0 && p(f, { direction: "row", gap: "5px", alignItems: "center", className: t.statItem, children: [n(W, { size: 14, icon: In[s] }), n(C, { variant: "body1", children: a })] }, s)), n("div", { className: t.verticalDivider })] }) : null;
}, Qe = B()((e, t, r) => ({ root: { "--stats-color": "transparent", "--menu-item-svg-color": "var(--secondary)", "--title-flex-shrink": 0, "--stats-divider-color": "transparent", "--stats-opacity": 0, "--stats-width": "0px", padding: "10px", borderRadius: "10px", [`&.${r.compact}`]: { padding: 0, height: 24 }, [`&.${r.selected}`]: { backgroundColor: "var(--list-hover-hover)", "--stats-color": "var(--secondary)", "--stats-opacity": 1, "--stats-width": "100%", "--title-flex-shrink": 1, "--stats-divider-color": "var(--light-grey-4)", "&:hover": { background: "var(--list-hover-hover)" }, ".termius-dark-theme &": { "--stats-divider-color": "var(--dark-grey-transparent)" } }, "&:hover": { backgroundColor: "var(--list-hover-hover)", "--stats-color": "var(--secondary)", "--stats-opacity": 1, "--stats-width": "100%", "--title-flex-shrink": 1, "--stats-divider-color": "var(--light-grey-4)", ".termius-dark-theme &": { backgroundColor: "var(--list-hover-hover)", borderColor: "var(--list-hover-hover)", "--stats-divider-color": "var(--dark-grey-transparent)" } }, [`&:hover .${r.statList}`]: { visibility: "visible" }, [`&:hover:not(.${r.selected})`]: { backgroundColor: "var(--list-hover-hover)", borderColor: "var(--list-hover-hover)" }, ".termius-dark-theme &": { boxShadow: "none", [`&:hover:not(.${r.selected})`]: { backgroundColor: "var(--list-hover-hover)", borderColor: "var(--list-hover-hover)" } } }, selected: {}, leftSection: { display: "flex", alignItems: "center", overflow: "hidden", gap: "5px", paddingRight: "10px" }, rightSection: { display: "flex", alignItems: "center", flexShrink: 0, position: "relative" }, statList: { visibility: "hidden", transition: "opacity 0.1s ease-in", opacity: "var(--stats-opacity)", maxWidth: "var(--stats-width)", [`.${r.selected} &`]: { visibility: "visible" } }, statItem: { color: "var(--stats-color)" }, verticalDivider: { height: "12px", width: "1px", backgroundColor: "var(--stats-divider-color)", marginRight: "10px" }, compact: {} })), X = { keys: ["full_name", "email"], threshold: H.rankings.CONTAINS }, lr = { ...X, keys: ["name"], sorter: (e) => e }, cr = { type: "divider" }, dr = (e, t, r, s) => {
  const a = H(e, t.inputValue, X), i = H(r, t.inputValue, X), o = H(s, t.inputValue, lr);
  return ur(o, a, i);
}, ur = (e, t, r) => !e.length && !t.length && !r.length ? [{ type: "noOptions" }] : [...e, ...e.length && t.length ? [cr] : [], ...t], ae = B()((e, t, r) => ({ emailInput: { "--border-color": "var(--card-b)" }, listContainer: { backgroundColor: "var(--card-a)", padding: "10px", borderRadius: "15px" }, vaultsListTitle: { padding: "5px 0 5px 5px" }, emailNotFoundPaper: { padding: "5px 5px 5px 10px" }, teamPageLink: { color: "var(--info-base)", textDecoration: "none" }, option: { cursor: "pointer", padding: "5px", margin: "0", borderRadius: "10px", [`&:hover:not(.${r.optionDisabled})`]: { backgroundColor: "var(--list-hover-hover)" }, [`&.${J.focused}, &.${J.focused}[aria-selected="true"]`]: { backgroundColor: "var(--list-hover-hover)" } }, vaultOption: { padding: "8px" }, optionDisabled: { cursor: "not-allowed" }, paper: { boxShadow: "0px 15px 30px 0px var(--black-a15)", borderRadius: "10px" }, noOptions: { padding: "5px 10px 5px 5px" }, popperRoot: { "--scrollbar-width": 0, borderRadius: 15, width: 280, boxShadow: "0px 15px 30px 0px var(--black-a15)", backgroundColor: "var(--card-a)" }, externalLinkIcon: { width: 14, height: 14, color: "var(--secondary)" }, optionDivider: { height: 1, backgroundColor: "var(--background-entity)", margin: "10px" }, fullNameOption: { margin: "5px 0", width: "initial", "&:first-of-type": { marginTop: 0 }, [`&.${J.focused}, &.${J.focused}[aria-selected="true"]`]: { backgroundColor: "var(--list-hover-hover)" } }, emailSecondary: { fontSize: "12px" }, fullNameOptionPending: { cursor: "not-allowed", "&:last-of-type": { marginBottom: 0 } } })), Je = ({ members: e }) => {
  const { classes: t, cx: r } = ae();
  return p(_.Fragment, { children: [n(C, { variant: "body2", color: "secondary", className: t.vaultsListTitle, children: "Pending" }), e.map((s) => n(ze, { user: s, className: r(t.fullNameOption, t.fullNameOptionPending), subTitleClassName: t.emailSecondary }, s.id))] });
}, mr = _.forwardRef((e, t) => {
  const { classes: r } = ae(), { sentInvitesMembers: s, showVaults: a, showInvites: i, children: o, ...d } = e, l = !(Array.isArray(o) && o.some((u) => u.key === "noOptions"));
  return p("div", { className: r.listContainer, children: [a && n(C, { variant: "body2", color: "secondary", className: r.vaultsListTitle, children: "Copy from" }), p("ul", { ...d, ref: t, style: { padding: 0 }, children: [o, i && s.length > 0 && l && p(_.Fragment, { children: [n("div", { className: r.optionDivider }), n("ul", { children: n(Je, { members: s }) })] })] })] });
}), pr = ({ inputValue: e, alreadySelected: t }) => {
  const { classes: r, cx: s } = ae(), a = w(Z), i = w(Te);
  return !a || !i || t ? n(C, { variant: "body1", color: "secondary", ellipsis: true, className: r.emailNotFoundPaper, children: "No matching members found" }) : p(f, { direction: "row", justifyContent: "space-between", alignItems: "center", role: "button", overflow: "hidden", textOverflow: "ellipsis", onClick: pe, className: s(r.option, r.emailNotFoundPaper), children: [p(C, { variant: "body1", color: "secondary", children: ["Invite", " ", n(C, { variant: "body1", color: e ? "main" : "secondary", display: "inline", children: e || "new member" })] }), i && n(Me, { className: r.externalLinkIcon })] });
}, hr = ({ memberSuggestions: e, selectedMembers: t, onSelectedMembers: r, allVaults: s, currentVault: a, disabled: i = false }) => {
  const { classes: o, cx: d } = ae(), l = w(Ve), { open: u, inputValue: c, sentInvitesMembers: m, vaultOptionRef: h, vaultsOptionsList: S, sentInvitesMatch: v, handleOpen: x, handleClose: b, handleChange: I, handleInputChange: N, handleVaultSelect: M } = gr({ memberSuggestions: e, selectedMembers: t, onSelectedMembers: r, allVaults: s, currentVault: a });
  return n(xt, { classes: { paper: o.paper, noOptions: o.noOptions, option: o.option }, disabled: i, open: u, onOpen: x, onClose: b, disableClearable: true, forcePopupIcon: false, options: e, inputValue: c, autoSelect: false, autoHighlight: true, multiple: true, renderTags: () => null, value: t, filterSelectedOptions: true, filterOptions: (k, g) => dr(k, g, m, S), onChange: I, PopperComponent: (k) => n(wt, { ...k, placement: "left-start", style: { borderRadius: 15 }, id: "vaults-autocomplete-popper" }), PaperComponent: (k) => n("div", { ...k, className: o.popperRoot }), ListboxComponent: mr, ListboxProps: { sentInvitesMembers: m, showVaults: !c, showInvites: v.length > 0 }, noOptionsText: n(Je, { members: v }), renderInput: (k) => n(Re, { ...k, size: "medium", autoComplete: "off", variant: "outlined", label: "Add members by name or email", className: d(o.emailInput, k.inputProps.className) }), onInputChange: N, isOptionEqualToValue: (k, g) => "user_id" in k && "user_id" in g ? k.user_id === g.user_id : false, renderOption: (k, g) => {
    switch (g.type) {
      case "divider":
        return n("div", { className: o.optionDivider }, g.type);
      case "noOptions": {
        const T = c ? H(t, c, X).length > 0 : false;
        return n(pr, { inputValue: c, alreadySelected: T }, g.type);
      }
      case "vault":
        return O.createElement("li", { ...k, className: d(o.option, o.vaultOption), ref: h, key: g.id }, n(de, { variant: "compact", vault: g, membersCount: g.members.length, onClick: () => {
          M(g);
        } }));
      default:
        return O.createElement(ze, { ...k, key: g.user_id, user: g, avatar: _e(l, g.user_id), className: o.fullNameOption, subTitleClassName: o.emailSecondary });
    }
  }, getOptionLabel: () => "" });
}, gr = ({ memberSuggestions: e, selectedMembers: t, onSelectedMembers: r, allVaults: s, currentVault: a }) => {
  const i = $(), o = w(Le), [d, l] = _.useState(""), [u, c] = O.useState(false), [m, h] = _.useState(o), S = _.useRef(null), v = w(ne).find((E) => E.id === (a == null ? void 0 : a.id)), x = (E) => {
    const y = H(o, E || d, X);
    h(y);
  }, b = O.useMemo(() => Array.from(s.values()).filter((E) => F(E) ? false : oe(E).length > 0 && E.id !== (a == null ? void 0 : a.id)).map((E) => ({ ...E, type: "vault", members: oe(E) })), [s, a == null ? void 0 : a.id]), I = (E) => {
    const y = E.map((D) => ({ ...D, role: "Editor" }));
    E.filter((D) => !t.some((V) => V.user_id === D.user_id)).forEach((D) => {
      i(L("vaultMemberAdded", { isSearched: d ? "yes" : "no", vaultID: a == null ? void 0 : a.id, teamMemberID: D.email, hasVaultDataAccess: v ? "no" : "yes" }));
    }), r(ve([...t, ...y], "user_id"));
  }, N = (E, y, A, D) => {
    if (E.stopPropagation(), typeof y == "string" || !y) return;
    const V = (D == null ? void 0 : D.option.type) === "vault", z = y.find((q) => q.type === "noOptions");
    if (V && E.code === "Enter") {
      M(D == null ? void 0 : D.option);
      return;
    }
    if (z) {
      pe();
      return;
    }
    !V && !z && I(y);
  }, M = (E) => {
    const y = E.members.map((A) => ({ ...A, role: A.role === "Owner" ? "Editor" : A.role })).filter((A) => e.some((D) => D.user_id === A.user_id));
    r(ve([...y], "user_id")), c(false), i(L("copyVaultMembersButtonPressed", { vaultID: a == null ? void 0 : a.id, sourceVaultID: E.id, isDefaultVault: a != null && a.isDefault ? "yes" : "no", numberOfParticipants: E.members.length, isSearched: d ? "yes" : "no", hasVaultDataAccess: v ? "no" : "yes" }));
  };
  return { open: u, inputValue: d, sentInvitesMembers: o, sentInvitesMatch: m, vaultOptionRef: S, vaultsOptionsList: b, handleOpen: () => {
    x(), c(true), i(L("addingMembersToVaultStarted", { vaultID: a == null ? void 0 : a.id, hasVaultDataAccess: v ? "no" : "yes" }));
  }, handleClose: () => {
    c(false);
  }, handleChange: N, handleInputChange: (E, y, A) => (A === "reset" && l(""), x(y), l(y)), handleVaultSelect: M };
}, fr = ({ variant: e, onRemoveAccess: t, onInviteNewMember: r }) => {
  const { classes: s } = vr();
  return n(_.Fragment, { children: p(he, { children: [n(P, { when: e === "enabled", children: n(Ct, { className: s.removeButton, role: "button", onClick: t, tabIndex: 0, children: n(f, { direction: "row", justifyContent: "space-between", alignItems: "center", width: "100%", children: n(C, { variant: "body1", color: "error", children: "remove access" }) }) }) }), n(P, { when: e === "disabled", children: p(f, { gap: "5px", children: [n(f, { direction: "row", justifyContent: "space-between", alignItems: "center", width: "100%", padding: "0 10px", onClick: (a) => a.stopPropagation(), children: n(C, { variant: "body1", color: "secondary", className: s.teamVaultRemoveAccessLabel, children: "remove access" }) }), n("div", { className: s.addNewVaultContainer, children: p(C, { variant: "body2", color: "secondary", children: [n(C, { variant: "body2", display: "inline", children: n(Et, { color: "accent", onClick: r, children: "Add new vault" }) }), " ", "to set granular permissions"] }) })] }) })] }) });
}, ee = { hoverLight: "var(--light-grey-transparent-10)", hoverDark: "var(--dark-grey-transparent-10)" }, vr = B()({ removeButton: { color: "var(--dark-grey-6)", borderRadius: "10px", padding: "10px", ".termius-dark-theme &": { color: "var(--white)" }, "&.Mui-selected": { background: ee.hoverLight, "&:hover": { background: ee.hoverLight } }, "&.Mui-focusVisible": { background: ee.hoverLight }, "&:hover": { borderRadius: "10px", ".termius-dark-theme &": { backgroundColor: ee.hoverDark } } }, teamVaultRemoveAccessLabel: { color: "var(--light-grey-3)", ".termius-dark-theme &": { color: "var(--dark-grey-6)" } }, addNewVaultContainer: { height: "46px", padding: "5px 10px", background: "var(--background)", borderRadius: "10px" } }), br = (e) => e === "Editor" || e === "Owner" ? "can edit" : "can view", et = ({ member: e, onRoleChange: t, additionalSelectElement: r, disabled: s = false }) => {
  const { classes: a } = tt(), i = w(Ve);
  return n(St, { position: "relative", component: "li", children: p(f, { direction: "row", justifyContent: "space-between", alignItems: "center", gap: "10px", className: a.root, children: [p(f, { direction: "row", gap: "10px", alignItems: "center", overflow: "hidden", children: [n(kt, { avatar: _e(i, e.user_id), color: At(e.email, e.user_id ?? 0), letter: It({ email: e.email }), size: "medium" }), n(Dt, { title: e.email, placement: "top", tooltipClassName: a.emailTooltip, PopperProps: { modifiers: [{ name: "offset", options: { offset: [0, -8] } }] }, children: n("span", { className: a.email, children: n(C, { color: "primary", fontSize: 14, ellipsis: true, children: e.full_name || e.email }) }) })] }), n(yr, { member: e }), n(f, { children: t ? e.role !== "Owner" && n(On, { value: e.role, disabled: s, onChange: t, additionalSelectElement: r, renderLabel: (o) => o === "Editor" ? "can edit" : "can view" }) : n(C, { variant: "body1", color: "secondary", children: br(e.role) }) })] }) }, e.user_id);
}, yr = ({ member: e }) => {
  var a;
  const { classes: t } = tt(), r = w(Ot), s = [];
  return r.account.user_id === e.user_id && s.push(n(f, { className: t.ownerAdditionalSectionWrapper, children: n(C, { fontSize: 7, bolder: true, className: t.ownerAdditionalSectionLabel, children: "YOU" }) }, "you")), ((a = r.team) == null ? void 0 : a.owner_id) === e.user_id && s.push(n(f, { className: t.ownerAdditionalSectionWrapper, children: n(Dn, {}) }, "owner")), s.length === 0 ? null : n(f, { direction: "row", gap: "5px", children: s });
}, tt = B()({ root: { padding: "5px", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", borderRadius: "15px", height: "34px" }, email: { overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap", maxWidth: "150px" }, ownerAdditionalSectionWrapper: { justifyContent: "center", alignItems: "center", width: "25px", height: "15px", background: "var(--gradient-light-main)", borderRadius: "4px", ".termius-dark-theme &": { background: "var(--main-gradient-outline)" } }, ownerAdditionalSectionLabel: { background: "var(--gradient-universal-main)", WebkitBackgroundClip: "text", WebkitTextFillColor: "transparent", textFillColor: "transparent" }, emailTooltip: { userSelect: "text" } }), Ee = { borderLight: "var(--light-grey-5)", borderDark: "var(--dark-grey-1)" }, nt = B()({ inviteForm: { overflowY: "hidden", height: "100%" }, addMembersSection: { margin: "0 15px", padding: "15px 20px 20px 20px", backgroundColor: "var(--card-a)", borderRadius: "15px", overflow: "auto" }, submitButtonWrapper: { marginTop: "10px", padding: "15px 15px 5px 15px", borderTop: `1px solid ${Ee.borderLight}`, ".termius-dark-theme &": { borderTop: `1px solid ${Ee.borderDark}` } }, submitButton: { flexShrink: 0 }, dividerLine: { ".termius-dark-theme &": { backgroundColor: "var(--background-entity)" } } }), xr = () => {
  var s;
  const { classes: e } = nt(), t = w(Le), r = ((s = w(De)) == null ? void 0 : s.id) ?? Oe;
  return t.length ? p(_.Fragment, { children: [n(Mn, { className: e.dividerLine }), n(f, { direction: "row", alignItems: "center", gap: "5px", children: n(C, { variant: "subtitle1", color: "main", children: "Pending" }) }), n(f, { component: "ul", gap: "10px", children: t.map((a, i) => n(et, { member: { id: a.id, role: a.role, vault_id: r, user_id: a.user_id, email: a.email, full_name: a.full_name || "" }, additionalSelectElement: null }, i)) })] }) : null;
}, wr = (e) => {
  const { disabled: t, formControl: r } = e, s = $(), a = w(Pe), { field: i } = Be({ name: "selectedMembers", control: r }), { value: o } = i, d = (c) => {
    t || i.onChange(c);
  };
  return { selectedMembers: o, applyUpdate: d, updateMemberRole: (c, m) => {
    const h = o.map((S) => S.user_id === c.user_id ? { ...S, role: m } : S);
    d(h), s(L("vaultPermissionsChanged", { permissionPickerLocation: "Edit Vault Screen", vaultPermissions: Cr(m), teamMemberID: c.email, vaultID: c.vault_id, hasVaultDataAccess: a.includes(c.vault_id) ? "no" : "yes" }));
  }, removeSelectedMember: (c) => {
    s(L("vaultPermissionsChanged", { permissionPickerLocation: "Edit Vault Screen", vaultPermissions: "No access", teamMemberID: c.email, vaultID: c.vault_id, hasVaultDataAccess: a.includes(c.vault_id) ? "no" : "yes" }));
    const m = o.filter((h) => h.user_id !== c.user_id);
    return d(m);
  } };
}, Cr = (e) => e ? e === "Editor" || e === "Owner" ? "Can edit" : "Can view" : "No access", Er = (e) => {
  const { allVaults: t, selectedVault: r, onAddVaultClick: s, teamOwner: a, userId: i, disabled: o } = e, { permissions: d } = r, l = d.has("editMembersList"), u = w(Pn), { selectedMembers: c, applyUpdate: m, updateMemberRole: h, removeSelectedMember: S } = wr(e), v = c.some((b) => b.user_id === a.userId), x = (b, I) => {
    I !== "no access" && h(b, I);
  };
  return p(_.Fragment, { children: [d.has("editMembersList") && n(hr, { memberSuggestions: Se(u), allVaults: t, currentVault: r, selectedMembers: Se(c), onSelectedMembers: m, disabled: o }), !v && a.userId !== i && n(Ke, { variant: "withOwnerAccess" }), c.length > 0 && n(f, { component: "ul", gap: "10px", children: c.map((b) => n(et, { disabled: o, member: b, onRoleChange: d.has("updateMembersRoles") ? (I) => x(b, I) : void 0, additionalSelectElement: n(fr, { variant: l ? "enabled" : "disabled", onInviteNewMember: s, onRemoveAccess: l ? () => S(b) : void 0 }) }, b.user_id)) }), r.isDefault && n(xr, {})] });
}, Se = (e) => e.map((t) => ({ ...t, type: "member" })), Sr = (e) => {
  const { selectedVault: t, ...r } = e, { classes: s } = nt();
  if (!t) return null;
  const i = !F(t) && me(t), d = t.isDefault && Fe(t), l = !i && !d, { submitButton: u, handleSubmit: c } = e.saving;
  return p(f, { component: "form", justifyContent: "space-between", className: s.inviteForm, onSubmit: c, "data-testid": "vault-edit-form", children: [n(f, { className: s.addMembersSection, children: p(f, { gap: "10px", children: [n(C, { variant: "subtitle1", color: "main", children: d ? "No access to this vault" : "People with access to this vault" }), n(Ke, { variant: kr(t) }), l && n(Er, { ...r, selectedVault: t })] }) }), u && n(f, { className: s.submitButtonWrapper, children: n(Y, { className: s.submitButton, color: "accent", size: "medium", type: "submit", fullWidth: true, disabled: u.disabled, children: u.label }) })] });
}, kr = (e) => {
  const r = !F(e) && me(e), s = e.isDefault, a = s && Fe(e);
  return r ? "personal" : a ? "noAccess" : s ? "team" : "hidden";
}, Ar = ({ formControl: e, canEdit: t, disabled: r, forbiddenNames: s, vaultNameInputRef: a, onSubmit: i }) => {
  var S, v;
  const { classes: o, cx: d } = Ir(), [l, u] = O.useState(true), c = (x) => !s.includes(x.trim()) || "This name is already in use", { field: m, fieldState: h } = Be({ name: "name", control: e, rules: { required: "Vault name is required", validate: t ? { notAllowed: c } : void 0 } });
  return p("header", { className: o.header, children: [n("form", { className: o.form, onSubmitCapture: (x) => {
    i(), x.preventDefault();
  }, onSubmit: i, children: n(f, { direction: "row", gap: "5px", alignItems: "center", width: "100%", children: p(_.Fragment, { children: [n(W, { icon: n(Ne, { width: "20px", height: "20px", color: "var(--main)" }), size: 36 }), p(he, { children: [n(P, { when: t, children: n(Re, { name: "name", title: "Vault name input field", disabled: r, inputRef: a, inputProps: { ...m }, autoFocus: true, type: "text", onBlur: () => {
    u(false);
  }, onFocus: () => u(true), placeholder: "Vault name", className: d({ [o.nameEdit]: true, [o.nameEditIdle]: !l, [o.errorField]: !!((S = h.error) != null && S.message) }), onKeyDown: (x) => {
    x.key === "Escape" && (x.preventDefault(), x.stopPropagation(), x.target instanceof HTMLInputElement && (x == null || x.target.blur()));
  } }) }), n(P, { when: !t, children: n(C, { className: o.nameStatic, variant: "mediumBody1", color: "main", title: "Vault name static", ellipsis: true, children: m.value }) })] })] }) }) }), n(Ue, { in: !!h.error, children: n("div", { className: o.error, children: n(Mt, { skipInternalMargin: true, status: "error", children: (v = h.error) == null ? void 0 : v.message }) }) })] });
}, Ir = B()({ header: { padding: "15px 20px 15px 10px", margin: "0 15px 15px 15px", borderRadius: "15px", backgroundColor: "var(--card-a)" }, form: { display: "flex", justifyContent: "space-between", alignItems: "center" }, nameStatic: { marginLeft: "0px" }, nameEdit: { width: "100%" }, nameEditIdle: { "--border-color": "var(--card-b)", whiteSpace: "nowrap", textOverflow: "ellipsis", overflow: "hidden", background: "transparent", "&:hover": { borderColor: "var(--light-grey-3)" } }, errorField: { borderColor: "var(--error-medium)" }, error: { paddingLeft: 4, marginTop: 5 } }), rt = (e) => {
  const { children: t } = e;
  return n(f, { paddingX: "15px", children: n(f, { alignItems: "center", gap: "10px", width: "100%", padding: "40px 20px 20px 20px", borderRadius: "15px", bgcolor: "var(--card-a)", children: t }) });
}, at = (e) => {
  const { icon: t, title: r, description: s, action: a } = e;
  return p(rt, { children: [n(W, { size: 64, icon: t }), p(f, { gap: "5px", alignItems: "center", textAlign: "center", children: [n(C, { variant: "h3", h3Margin: true, color: "main", children: r }), s && n(C, { variant: "body1", color: "secondary", children: s })] }), n(f, { gap: "10px", width: "100%", marginTop: "20px", children: a })] });
}, Dr = ({ action: e }) => n(at, { icon: n(Nt, { color: "var(--light-grey-3)" }), title: "Connection error", description: p(O.Fragment, { children: ["Please check your internet connection", n("br", {}), " and try again"] }), action: e }), Or = ({ action: e, title: t }) => n(at, { icon: n(Tt, { color: "var(--text-error)" }), title: t, action: e }), ke = (e) => {
  const { title: t, description: r } = e;
  return p(rt, { children: [n(Nn, { width: "64px", height: "64px" }), p(f, { gap: "5px", alignItems: "center", textAlign: "center", children: [n(C, { variant: "h3", h3Margin: true, color: "main", children: t }), r && n(C, { variant: "body1", color: "secondary", children: r })] })] });
}, Mr = n(C, { color: "error", component: "span", children: "Delete vault" }, "removeVault"), Nr = ({ open: e, title: t, onCloseStart: r, onCloseEnd: s, onDelete: a, children: i, disabled: o = false }) => {
  const { classes: d } = Tr(), l = Vt(), u = w((h) => Rt(h, $e())), c = [{ name: Mr, handler: () => {
    a == null || a(), l.closeMenu();
  } }];
  _t("keydown", (h) => {
    h.key === "Escape" && (r == null || r(h));
  });
  const m = () => {
    const h = document.getElementById("vaults-autocomplete-popper");
    !e || h || u.length || r == null || r();
  };
  return n(Lt, { className: d.sidebar, isOpened: e, onCloseAnimationEnd: s, children: n(Tn, { onClickAway: m, mouseEvent: "onMouseDown", children: p("div", { className: d.form, children: [n(Pt, { left: n(Bt, { title: t }), right: p(Ft, { margin: "md", children: [a && n(Ut, { ref: l.triggerRef, onClick: l.openMenu, disabled: o, "aria-label": "Extra actions", children: n($t, {}) }), n(Wt, { disabled: o, "data-testid": "form-close-btn", onClick: (h) => r == null ? void 0 : r(h) })] }) }), n(jt, { onClose: l.closeMenu, autoClose: true, open: l.isOpened, anchorEl: l.triggerRef.current, anchorOrigin: { horizontal: "right", vertical: "bottom" }, transformOrigin: { horizontal: "right", vertical: "top" }, menuItems: c }), i] }) }) });
}, Ae = { borderLight: "var(--light-grey-5)", borderDark: "var(--dark-grey-1)" }, Tr = B()({ sidebar: { marginTop: "calc(0px - var(--header-height))" }, form: { ".termius-dark-theme &": { background: "var(--dark-grey-2)", borderLeft: `1px solid ${Ae.borderDark}` }, flex: 1, overflowY: "hidden", display: "flex", flexDirection: "column", background: "var(--light-grey-6)", borderLeft: `1px solid ${Ae.borderLight}` } }), Vr = ({ selectedVault: e, allVaults: t, originalVaults: r, open: s, xstate: a, form: i, onCloseStart: o, onCloseEnd: d, onSubmit: l, onAddVaultClick: u, vaultNameInputRef: c }) => {
  const { state: m, send: h } = a, { control: S } = i, v = w(ue), x = w(We), b = w(ge), I = w(fe), N = $(), M = O.useMemo(() => x == null || v == null || v.owner_id == null || v.owner == null, [v, x]);
  O.useEffect(() => {
    M && N(Ht("/account"));
  }, [N, M]);
  const { title: k, submitButton: g, formState: T, vaultName: E } = Pr({ userId: x, username: b, userFullName: I, form: i, selectedVault: e, allVaults: t, xstate: a, originalVaults: r }), { hasDeletePermission: y, deleteButtonAvailable: A, vaultEditDisabled: D } = Lr(e, m, T), V = () => m.matches("deleting") ? "Couldn't delete vault" : m.context.SAVE_MODE === "create" ? "Couldn't create vault" : "Couldn't update vault", z = _r(s, o, T), K = r.get((e == null ? void 0 : e.id) ?? 0), q = (m.matches("idle") || m.matches("saving.persisting") || m.matches("saving.updateSuccess")) && !!e;
  return n(Nr, { open: s, title: k, disabled: D, onCloseStart: z, onCloseEnd: d, onDelete: A ? () => h("DELETE") : void 0, children: n(f, { direction: "column", justifyContent: "flex-start", padding: "15px 0px 10px 0px", width: "552px", height: "100%", maxWidth: "100%", overflow: "auto", children: p(he, { children: [n(P, { when: q, children: n(Rr, { userId: x, vaultName: E, vaultNameInputRef: c, allVaults: t, selectedVault: e, formControl: S, formState: T, submitButton: g, onSubmit: l, onAddVaultClick: u, teamInfo: v }) }), n(P, { when: m.matches("saving.createSuccess"), children: n(ke, { title: "The vault has been created!", description: `Now you can move your entities to it
or create new in this vault to collaborate.` }) }), n(P, { when: (m.matches("deleting.prompt") || m.matches("deleting.requesting")) && !!K, children: K && n(Bn, { vault: K, progress: m.hasTag("pending"), onDelete: y ? () => h("CONFIRM_DELETE") : void 0, onClose: y ? () => h("CANCEL_DELETE") : void 0 }) }), n(P, { when: m.matches("deleting.success"), children: n(ke, { title: "The vault has been deleted!" }) }), n(P, { when: m.matches("saving.genericError") || m.matches("deleting.genericError"), children: n(Or, { title: V(), action: n(Y, { onClick: () => h("RESET"), variant: "contained", color: "accent", fullWidth: true, children: "Back" }) }) }), n(P, { when: m.matches("saving.networkError") || m.matches("deleting.networkError"), children: n(Dr, { action: p(O.Fragment, { children: [n(Y, { onClick: () => !T.pending && h("RETRY"), variant: "contained", color: "accent", fullWidth: true, children: "Try again" }), n(Y, { onClick: () => h("RESET"), variant: "text", fullWidth: true, children: "Back" })] }) }) })] }) }) });
}, Rr = ({ formControl: e, vaultNameInputRef: t, vaultName: r, formState: s, allVaults: a, submitButton: i, userId: o, selectedVault: d, onSubmit: l, onAddVaultClick: u, teamInfo: c }) => !((c == null ? void 0 : c.owner_id) != null && (c == null ? void 0 : c.owner) != null) || !o ? null : p(_.Fragment, { children: [n(Ar, { formControl: e, vaultNameInputRef: t, canEdit: r.canEdit, disabled: r.disabled, forbiddenNames: r.forbiddenNames, onSubmit: () => {
  s.dirty && s.isValid && l();
} }), n(Sr, { allVaults: a, disabled: s.pending, saving: { handleSubmit: l, submitButton: i }, userId: o, teamOwner: { userId: c.owner_id, email: c.owner, fullName: c.owner_name }, selectedVault: d, onAddVaultClick: u, formControl: e })] }), _r = (e, t, r) => (s) => {
  if (!e) return;
  const a = r.dirty, i = (s == null ? void 0 : s.type) === "click" || (s == null ? void 0 : s.key) === "Escape";
  t({ isDirty: a, forceClose: i || void 0 });
}, Lr = (e, t, r) => {
  var o;
  const s = !!((o = e == null ? void 0 : e.permissions) != null && o.has("removeVault")), a = s && !t.matches("saving.createSuccess") && !t.matches("deleting") && !t.hasTag("errorNotification"), i = r.pending || t.matches("deleting.requesting");
  return { hasDeletePermission: s, deleteButtonAvailable: a, vaultEditDisabled: i };
}, Pr = ({ userId: e, username: t, userFullName: r, form: s, selectedVault: a, allVaults: i, xstate: o, originalVaults: d }) => {
  const { state: l } = o, u = (a == null ? void 0 : a.permissions) ?? /* @__PURE__ */ new Set(), c = Array.from(i.values()).filter((y) => y.id !== (a == null ? void 0 : a.id)).map((y) => y.name), m = !!(a && F(a)), h = u.has("editName") || u.has("editMembersList") || u.has("updateMembersRoles"), S = u.has("editName"), v = (y) => y.map((A) => ({ user_id: A.user_id, role: A.role })).sort((A, D) => A.user_id - D.user_id), x = O.useMemo(() => {
    if (!(a != null && a.id) || !s.formState.isDirty) return false;
    const y = d.get(a.id);
    if (!y || !e) return false;
    const A = { members: v(le({ vault: y, shouldCreate: m, userId: e, username: t, userFullName: r })), name: y.name }, D = { members: v(a.members), name: a.name };
    return !be(A, D);
  }, [s.formState.isDirty, m, d, a, e, t, r]), b = O.useMemo(() => {
    if (!e) return false;
    const y = { members: v(le({ vault: U, shouldCreate: m, userId: e, username: t, userFullName: r })), name: U.name }, A = { members: v((a == null ? void 0 : a.members) ?? []), name: a == null ? void 0 : a.name };
    return !be(y, A);
  }, [m, a == null ? void 0 : a.members, a == null ? void 0 : a.name, e, t, r]), I = (() => l.matches("deleting") ? "Delete this vault" : m ? "New vault" : "Vault details")(), N = (y, A) => y ? m ? "Create vault" : "Saving changes" : A ? "Changes saved" : m ? "Create vault" : "Save changes", M = (y, A) => y ? n(qt, { thickness: 5, size: 14, color: "inherit" }) : A ? n(Yt, { width: "14px", height: "14px" }) : null, k = (() => {
    const y = l.hasTag("pending") && l.matches("saving"), A = l.matches("saving.updateSuccess"), D = N(y, A), V = M(y, A);
    return V ? p(f, { direction: "row", gap: "10px", alignItems: "center", color: "'var(--light-grey-3)'", children: [V, n(C, { variant: "subtitle2", children: D })] }) : D;
  })(), g = { dirty: m ? s.formState.isDirty && !l.hasTag("pending") && b : s.formState.isDirty && !l.hasTag("pending") && !l.matches("saving.updateSuccess") && x, pending: l.hasTag("pending") && l.matches("saving"), isValid: s.formState.isValid }, T = h ? { label: k, disabled: !g.dirty || !g.isValid, progress: g.pending } : void 0, E = { canEdit: S, disabled: g.pending, forbiddenNames: zt.concat(c) };
  return { title: I, vaultName: E, submitButton: T, formState: g };
}, Br = (e, t) => O.createElement("svg", { width: 20, height: 20, viewBox: "0 0 20 20", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: t, ...e }, O.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M10 20C15.5228 20 20 15.5228 20 10C20 4.47715 15.5228 0 10 0C4.47715 0 0 4.47715 0 10C0 15.5228 4.47715 20 10 20ZM5.05025 8.9598L9.50502 4.50502C9.77839 4.23166 10.2216 4.23166 10.495 4.50502L14.9497 8.9598C15.2231 9.23317 15.2231 9.67638 14.9497 9.94975C14.6764 10.2231 14.2332 10.2231 13.9598 9.94975L10.7 6.68995L10.7 15C10.7 15.3866 10.3866 15.7 10 15.7C9.6134 15.7 9.3 15.3866 9.3 15V6.68995L6.0402 9.94975C5.76683 10.2231 5.32362 10.2231 5.05025 9.94975C4.77689 9.67638 4.77689 9.23316 5.05025 8.9598Z", fill: "currentColor" })), Fr = O.forwardRef(Br), Ur = O.memo(Fr), $r = ({ vaults: e, selectedVault: t, vaultNameInputRef: r, onClick: s, onVaultClick: a, onAddVaultClick: i }) => {
  const { classes: o } = st(), d = w(Z), [l, u] = O.useState(false), c = w(Pe), m = w(Kt), h = w(Te), S = w(Gt), v = $(), x = je(), b = d ? Xt(Array.from(e.values()), (g) => !(g.id && c.includes(g.id))) : [Array.from(e.values())], [I, N] = b, M = (g) => F(g) ? 0 : oe(g).length;
  O.useEffect(() => {
    const g = I.some((T) => T.id === U.id);
    u(g);
  }, [I]);
  const k = () => {
    S && v(L("newVaultButtonPressed", { newVaultSource: "Vaults Screen", eventTeamRole: S })), h ? i() : Qt(x, { dialog: "UpgradeToBusinessViaVaultsDialog", target: $e(), props: {} }).catch(Jt);
  };
  return p(f, { className: o.root, alignItems: "center", gap: "30px", onClick: s, children: [p(f, { className: o.vaults, component: "ul", children: [I.map((g) => {
    const T = F(g), E = n(de, { vault: g, selected: g.id === (t == null ? void 0 : t.id), onClick: () => {
      var y;
      return T ? (y = r.current) == null ? void 0 : y.focus() : a(g);
    }, membersCount: M(g) }, g.id);
    return T ? n(Ue, { in: l, children: E }, g.id) : E;
  }), m && p(_.Fragment, { children: [n("div", { className: o.divider }), n(Wr, { onClick: k, label: "Add vault", variant: h ? "default" : "withUpgradeIcon" })] })] }), d && N.length > 0 && p(f, { className: o.vaults, component: "ul", children: [n(C, { variant: "subtitle1", color: "secondary", className: o.listTitle, children: "Other vaults" }), N.map((g) => n(de, { vault: g, selected: g.id === (t == null ? void 0 : t.id), onClick: () => a(g), membersCount: M(g) }, g.id))] })] });
}, Wr = ({ onClick: e, label: t, variant: r }) => {
  const { classes: s } = st(), a = (i) => {
    i.stopPropagation(), e();
  };
  return p("div", { role: "button", className: s.newVaultButton, onClick: a, tabIndex: 0, children: [n(W, { size: 24, icon: n(Zt, { color: "var(--main)" }) }), p(f, { direction: "row", alignItems: "center", gap: "8px", children: [n(C, { variant: "body1", color: "primary", ellipsis: true, children: t }), r === "withUpgradeIcon" && n(Ur, { width: 14, height: 14, color: "var(--secondary)" })] })] });
}, st = B()({ root: { background: "var(--background)", padding: "30px 20px 30px 20px", width: "100%", height: "100%", overflowY: "auto", overflowX: "hidden" }, vaults: { width: "100%", maxWidth: "700px", listStyle: "none", margin: 0, padding: 15, borderRadius: 15, display: "flex", flexDirection: "column", gap: "5px", backgroundColor: "var(--card-a)", "@media only screen and (max-width: 1280px)": { maxWidth: "600px" }, "@media only screen and (max-width: 900px)": { maxWidth: "500px" }, "@media only screen and (max-width: 800px)": { width: "100%", margin: "30px 20px 0 20px" } }, listTitle: { padding: "0 15px" }, newVaultButton: { display: "flex", gap: "5px", alignItems: "center", alignSelf: "stretch", padding: "10px", borderRadius: "10px", "&:hover": { background: "var(--background)", borderColor: "var(--background)", ".termius-dark-theme &": { background: "var(--list-hover-hover)", borderColor: "var(--list-hover-hover)" } } }, divider: { height: "1px", width: "100%", backgroundColor: "var(--background-entity)" } }), jr = () => {
  const e = je();
  return { deleteVault: async (a) => se(e, nn, { vaultId: a }), createVaults: async (a) => se(e, en, a), updateVault: async (a, i, o, d, l) => (ce(a.id, "Vault id is required"), se(e, tn, { vaultName: i, vaultId: a.id, teamOwnerId: l, newVaultMembers: o, originalVaultMembers: d })) };
}, Hr = rn({ predictableActionArguments: true, preserveActionOrder: true, id: "vaultLifecycleFlow", initial: "idle", context: { SAVE_MODE: "create", result: null }, on: { RESET: { target: "idle" } }, states: { idle: { id: "idle", on: { CREATE: { target: "saving", actions: "setCreateMode" }, UPDATE: { target: "saving", actions: "setUpdateMode" }, DELETE: { target: "deleting", cond: "isDeleteAvailable" }, CLOSE: { target: "idle", actions: { type: "onDismiss" } } } }, saving: { initial: "persisting", states: { persisting: { tags: ["pending"], invoke: { src: "saveVault", id: "saveVaultActor", onDone: [{ target: "networkError", cond: "isNetworkErrorResult", actions: { type: "onSaveFailed" } }, { target: "genericError", cond: "isErrorResult", actions: { type: "onSaveFailed" } }, { target: "createSuccess", cond: (e) => e.SAVE_MODE === "create", actions: [{ type: "onSaveSuccess" }, ie({ result: (e, t) => t.data.val })] }, { target: "updateSuccess", cond: (e) => e.SAVE_MODE === "update", actions: { type: "onSaveSuccess" } }] } }, createSuccess: { after: { 2e3: [{ target: "#idle", cond: "hasVault" }, { target: "#closeTransition", actions: { type: "onDismiss" } }] }, on: { CLOSE: { target: "#idle" } } }, updateSuccess: { after: { 1e3: { target: "#idle" } }, on: { CLOSE: { target: "#idle" } } }, genericError: { tags: ["errorNotification"], on: { CLOSE: { target: "#idle", actions: { type: "onDismiss" } }, RETRY: { target: "persisting" } } }, networkError: { tags: ["errorNotification"], on: { CLOSE: { target: "#idle", actions: { type: "onDismiss" } }, RETRY: { target: "persisting" } } } } }, deleting: { initial: "prompt", states: { prompt: { on: { CONFIRM_DELETE: { target: "requesting" }, CANCEL_DELETE: { target: "#idle" }, CLOSE: { target: "#idle", actions: { type: "onDismiss" } } } }, requesting: { tags: ["pending"], invoke: { src: "deleteVault", id: "deleteVaultActor", onDone: [{ target: "networkError", cond: "isNetworkErrorResult", actions: { type: "onDeleteFailed" } }, { target: "genericError", cond: "isErrorResult", actions: { type: "onDeleteFailed" } }, { target: "success", actions: { type: "onDeleteSuccess" } }] } }, success: { after: { 2e3: { target: "#closeTransition", actions: { type: "onDismiss" } } }, on: { CLOSE: { target: "#closeTransition", actions: { type: "onDismiss" } } } }, genericError: { tags: ["errorNotification"], on: { RETRY: { target: "requesting" }, CLOSE: { target: "#idle", actions: { type: "onDismiss" } } } }, networkError: { tags: ["errorNotification"], on: { RETRY: { target: "requesting" }, CLOSE: { target: "#idle", actions: { type: "onDismiss" } } } } } }, closeTransition: { id: "closeTransition", after: { 200: { target: "#idle" } } } }, tsTypes: {}, schema: { context: {}, services: {}, events: {} } }, { services: {}, actions: { setCreateMode: ie({ SAVE_MODE: "create" }), setUpdateMode: ie({ SAVE_MODE: "update" }) }, guards: { isNetworkErrorResult: (e, { data: t }) => {
  var r;
  return t.err && ((r = t.val.message) == null ? void 0 : r.startsWith("Network error"));
}, isErrorResult: (e, t) => t.data.err, hasVault: (e) => {
  var t;
  return !!((t = e.result) != null && t.vault);
} } }), zr = "update", qr = "delete", Ie = "onChange";
function Yr(e) {
  const { userId: t, vault: r, onDismiss: s, onSuccess: a, allVaults: i } = e, o = w(ge), d = w(fe), l = w(an), u = r && F(r), { createVaults: c, updateVault: m, deleteVault: h } = jr(), S = { name: "", selectedMembers: [{ user_id: t, role: "Editor", email: o, full_name: d, vault_id: r == null ? void 0 : r.id }] }, v = sn({ defaultValues: S, reValidateMode: Ie, mode: Ie }), x = v.watch("name"), b = v.watch("selectedMembers"), [I, N] = on(Hr, { services: { saveVault: async () => {
    var k;
    if (l == null) return G({ message: "Team owner not found" });
    if (r == null) return G({ message: "Vault not found" });
    if (u) return c({ teamOwnerId: l, vaults: [{ vaultName: x, newVaultMembers: b }] });
    const M = (k = i.get(r.id)) == null ? void 0 : k.members.filter((g) => "id" in g);
    return M == null ? G({ message: "Original vault members were not found" }) : r.id == null ? G({ message: "Vault id not found" }) : m(r, x, b, M, l);
  }, deleteVault: async () => r == null || r.id == null ? G({ message: "Vault not found" }) : h(r.id) }, actions: { onSaveSuccess: (M, k) => {
    ce(k.data.ok, "Save vault failed"), a({ data: k.data.val, mode: zr });
  }, onSaveFailed() {
  }, onDeleteSuccess: (M, k) => {
    ce(k.data.ok, "Delete vault failed"), a({ data: k.data.val, mode: qr });
  }, onDeleteFailed() {
  }, onDismiss() {
    s();
  } }, guards: { isDeleteAvailable: () => r != null }, id: r == null ? "new-vault" : `edit-vault-${r.id}`, devTools: false, context: { SAVE_MODE: "create" } });
  return [I, N, v];
}
const Kr = (e) => {
  const { userId: t, form: r, xstate: s, sliderOpen: a, selectedVaultId: i } = e, o = $(), d = w(Z), l = w(ge), u = w(fe), c = w(ne), m = (b) => {
    if (!t) return;
    const I = { name: b.name, members: le({ vault: b, shouldCreate: b.id === U.id, userId: t, username: l, userFullName: u }), permissions: ln(b, t, d) };
    o(He(b.id)), o(te({ ...b, ...I })), r.reset({ name: I.name, selectedMembers: I.members });
  }, h = (b) => {
    s.state.hasTag("pending") || i === b.id || (s.send("RESET"), m(b));
  }, S = (b) => {
    o(L("editVaultScreenOpened", { isDefaultVault: b.isDefault ? "yes" : "no", vaultID: b.id, numberOfParticipants: b.members.length || 0, hasVaultDataAccess: c.find((I) => I.id === b.id) ? "no" : "yes" }));
  };
  return { handleVaultClick: (b) => {
    h(b), S(b);
  }, changeEditVaultData: h, handleVaultListClick: () => {
    !s.state.hasTag("pending") && a && o(j());
  }, setDraftAndSelectedVaultId: m };
}, Gr = (e) => {
  const { form: t, selectedVaultId: r, draft: s, xstate: { send: a } } = e, i = $(), o = w(ne), [d, l] = cn({ control: t.control, name: ["name", "selectedMembers"] }), u = t.handleSubmit(() => (i(L("saveVaultButtonPressed", { vaultID: s == null ? void 0 : s.id, numberOfParticipants: (s == null ? void 0 : s.members.length) || 0, hasVaultDataAccess: o.find((c) => c.id === (s == null ? void 0 : s.id)) ? "no" : "yes" })), r === U.id ? a("CREATE") : a("UPDATE")));
  return O.useEffect(() => {
    if (r) {
      if (!d) {
        i(te({ name: "", members: l }));
        return;
      }
      t.formState.isDirty && i(te({ name: d, members: l }));
    }
  }, [i, d, t.formState.isDirty, l, r]), { handleSubmit: u };
}, ia = () => {
  const { state: { sliderOpen: e, draft: t }, vaultsMap: r, presentedVaults: s, xstate: a, form: i, vaultNameInputRef: o, handleVaultClick: d, handleVaultListClick: l, handleCloseStart: u, handleCloseEnd: c, handleSubmit: m } = Xr();
  return p(dn, { fullHeight: true, children: [n(En, { children: n($r, { vaults: s, selectedVault: t, vaultNameInputRef: o, onClick: l, onVaultClick: d, onAddVaultClick: () => d(U) }) }), n(Vr, { selectedVault: t, allVaults: s, originalVaults: r, open: e, xstate: a, form: i, vaultNameInputRef: o, onCloseStart: u, onCloseEnd: c, onSubmit: m, onAddVaultClick: () => d(U) })] });
}, Xr = () => {
  const e = $(), t = w(un), r = w(We), s = w(Z), a = w(mn), { sliderOpen: i, selectedVaultId: o, openVaultRemotelyById: d, vaultDraft: l, openConfirmDialog: u } = w(Vn), c = _.createRef(), m = pn(u);
  hn(() => {
    e(L("vaultsScreenOpened"));
  }), gn(() => {
    e(j());
  });
  const h = ({ data: E, mode: y }) => {
    const { vaults: A, participants: D, vault: V } = E;
    e(yn({ items: A })), e(xn(D));
    const [z] = wn(A);
    if (y === "update" && V) {
      const q = Cn(z, D, r, s, a).get(V == null ? void 0 : V.id);
      q && N(q);
    }
    y === "update" && !V && (e(j()), x.reset(), v("CLOSE")), y === "delete" && (e(He(void 0)), e(te(void 0)));
  }, [S, v, x] = Yr({ userId: r, vault: l, allVaults: t, onDismiss: () => {
    e(j());
  }, onSuccess: h }), { handleVaultClick: b, handleVaultListClick: I, setDraftAndSelectedVaultId: N } = Kr({ userId: r, form: x, xstate: { state: S, send: v }, sliderOpen: i, selectedVaultId: o }), { handleSubmit: M } = Gr({ form: x, xstate: { state: S, send: v }, selectedVaultId: o, draft: l });
  O.useEffect(() => {
    l && o !== void 0 && e(fn(true));
  }, [l, o, e]);
  const k = O.useCallback(async ({ isDirty: E, forceClose: y }) => {
    if (!E && y) {
      e(j());
      return;
    }
    E && !S.hasTag("pending") && !m.current && (e(ye(true)), await vn({ title: "Discard changes?", text: "You have unsaved changes. Are you sure you want to discard them?", confirmButtonTitle: "Discard", cancelButtonTitle: "Keep editing", buttonColor: "error" }).finally(() => {
      e(ye(false));
    }), e(j()));
  }, [e, S, m]), g = () => {
    S.hasTag("pending") || (e(j()), x.reset(), v("CLOSE"));
  }, T = (() => {
    const E = new Map(t);
    return l && E.set(l.id, l), E;
  })();
  return O.useEffect(() => {
    d && (x.formState.isDirty ? k({ isDirty: true }) : b(U), e(bn(void 0)));
  }, [d]), { state: { draft: l, sliderOpen: i }, presentedVaults: T, xstate: { state: S, send: v }, form: x, vaultsMap: t, vaultNameInputRef: c, handleVaultClick: b, handleVaultListClick: I, handleCloseStart: k, handleCloseEnd: g, handleSubmit: M };
};
export {
  ia as Vaults,
  Xr as useVaultsList
};
