/*
 * Recovered Termius UI module: MigrateToNewCryptoSystemScreen-95c65207.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { aR as Ye, ac as Ze, r, y5 as qe, C as ze, E as Ge, bi as z, br as E, bP as K, bt as e, cy as _, bu as V, bR as he, jB as Ce, bO as se, dG as me, y6 as we, cv as B, bv as Je, y7 as Ke, y8 as Qe, ft as Pe, xj as te, wa as Ie, k1 as re, dl as fe, eS as Xe, y9 as et, ya as tt, ms as rt, yb as nt, cX as ae, cN as pe, sh as be, vN as st, ew as J, yc as at, g2 as ke, bq as ge, yd as He, ye as _e, yf as Oe, yg as de, e7 as ot, dE as Ee, bj as it, gO as ct, yh as lt, yi as ut, yj as dt, yk as ht, yl as mt, iK as ft, ym as pt, dK as Me, bT as gt, iD as yt, rG as vt, rw as Ct, yn as wt, yo as St } from "./reconnectSaga-3b11f62e.js";
import { A as ee } from "./AnimationHandler-1e0795eb.js";
import { t as bt } from "./transitionScreenWrapper-62515248.js";
import { F as kt } from "./FullscreenDialogFooter-3b4c7076.js";
import { F as Ne } from "./FullScreenDialogLayout-e1c06c79.js";
import { M as Fe } from "./passwordPrompt.react-d6fafff3.js";
import { P as Ae } from "./PasswordInput-9d57733c.js";
import { u as De } from "./useRequest-2d237c57.js";
var Le = { exports: {} };
(function(s, t) {
  (function(i, n) {
    s.exports = n();
  })(Ye, function() {
    var i = 1e3, n = 6e4, h = 36e5, u = "millisecond", v = "second", p = "minute", S = "hour", f = "day", m = "week", g = "month", M = "quarter", b = "year", k = "date", R = "Invalid Date", I = /^(\d{4})[-/]?(\d{1,2})?[-/]?(\d{0,2})[Tt\s]*(\d{1,2})?:?(\d{1,2})?:?(\d{1,2})?[.:]?(\d+)?$/, y = /\[([^\]]+)]|Y{1,4}|M{1,4}|D{1,2}|d{1,4}|H{1,2}|h{1,2}|a|A|m{1,2}|s{1,2}|Z{1,2}|SSS/g, $ = { name: "en", weekdays: "Sunday_Monday_Tuesday_Wednesday_Thursday_Friday_Saturday".split("_"), months: "January_February_March_April_May_June_July_August_September_October_November_December".split("_"), ordinal: function(d) {
      var o = ["th", "st", "nd", "rd"], a = d % 100;
      return "[" + d + (o[(a - 20) % 10] || o[a] || o[0]) + "]";
    } }, F = function(d, o, a) {
      var l = String(d);
      return !l || l.length >= o ? d : "" + Array(o + 1 - l.length).join(a) + d;
    }, O = { s: F, z: function(d) {
      var o = -d.utcOffset(), a = Math.abs(o), l = Math.floor(a / 60), c = a % 60;
      return (o <= 0 ? "+" : "-") + F(l, 2, "0") + ":" + F(c, 2, "0");
    }, m: function d(o, a) {
      if (o.date() < a.date()) return -d(a, o);
      var l = 12 * (a.year() - o.year()) + (a.month() - o.month()), c = o.clone().add(l, g), C = a - c < 0, w = o.clone().add(l + (C ? -1 : 1), g);
      return +(-(l + (a - c) / (C ? c - w : w - c)) || 0);
    }, a: function(d) {
      return d < 0 ? Math.ceil(d) || 0 : Math.floor(d);
    }, p: function(d) {
      return { M: g, y: b, w: m, d: f, D: k, h: S, m: p, s: v, ms: u, Q: M }[d] || String(d || "").toLowerCase().replace(/s$/, "");
    }, u: function(d) {
      return d === void 0;
    } }, N = "en", A = {};
    A[N] = $;
    var Y = "$isDayjsObject", Z = function(d) {
      return d instanceof q || !(!d || !d[Y]);
    }, P = function d(o, a, l) {
      var c;
      if (!o) return N;
      if (typeof o == "string") {
        var C = o.toLowerCase();
        A[C] && (c = C), a && (A[C] = a, c = C);
        var w = o.split("-");
        if (!c && w.length > 1) return d(w[0]);
      } else {
        var x = o.name;
        A[x] = o, c = x;
      }
      return !l && c && (N = c), c || !l && N;
    }, T = function(d, o) {
      if (Z(d)) return d.clone();
      var a = typeof o == "object" ? o : {};
      return a.date = d, a.args = arguments, new q(a);
    }, D = O;
    D.l = P, D.i = Z, D.w = function(d, o) {
      return T(d, { locale: o.$L, utc: o.$u, x: o.$x, $offset: o.$offset });
    };
    var q = (function() {
      function d(a) {
        this.$L = P(a.locale, null, true), this.parse(a), this.$x = this.$x || a.x || {}, this[Y] = true;
      }
      var o = d.prototype;
      return o.parse = function(a) {
        this.$d = (function(l) {
          var c = l.date, C = l.utc;
          if (c === null) return /* @__PURE__ */ new Date(NaN);
          if (D.u(c)) return /* @__PURE__ */ new Date();
          if (c instanceof Date) return new Date(c);
          if (typeof c == "string" && !/Z$/i.test(c)) {
            var w = c.match(I);
            if (w) {
              var x = w[2] - 1 || 0, H = (w[7] || "0").substring(0, 3);
              return C ? new Date(Date.UTC(w[1], x, w[3] || 1, w[4] || 0, w[5] || 0, w[6] || 0, H)) : new Date(w[1], x, w[3] || 1, w[4] || 0, w[5] || 0, w[6] || 0, H);
            }
          }
          return new Date(c);
        })(a), this.init();
      }, o.init = function() {
        var a = this.$d;
        this.$y = a.getFullYear(), this.$M = a.getMonth(), this.$D = a.getDate(), this.$W = a.getDay(), this.$H = a.getHours(), this.$m = a.getMinutes(), this.$s = a.getSeconds(), this.$ms = a.getMilliseconds();
      }, o.$utils = function() {
        return D;
      }, o.isValid = function() {
        return this.$d.toString() !== R;
      }, o.isSame = function(a, l) {
        var c = T(a);
        return this.startOf(l) <= c && c <= this.endOf(l);
      }, o.isAfter = function(a, l) {
        return T(a) < this.startOf(l);
      }, o.isBefore = function(a, l) {
        return this.endOf(l) < T(a);
      }, o.$g = function(a, l, c) {
        return D.u(a) ? this[l] : this.set(c, a);
      }, o.unix = function() {
        return Math.floor(this.valueOf() / 1e3);
      }, o.valueOf = function() {
        return this.$d.getTime();
      }, o.startOf = function(a, l) {
        var c = this, C = !!D.u(l) || l, w = D.p(a), x = function(X, U) {
          var G = D.w(c.$u ? Date.UTC(c.$y, U, X) : new Date(c.$y, U, X), c);
          return C ? G : G.endOf(f);
        }, H = function(X, U) {
          return D.w(c.toDate()[X].apply(c.toDate("s"), (C ? [0, 0, 0, 0] : [23, 59, 59, 999]).slice(U)), c);
        }, L = this.$W, W = this.$M, j = this.$D, ne = "set" + (this.$u ? "UTC" : "");
        switch (w) {
          case b:
            return C ? x(1, 0) : x(31, 11);
          case g:
            return C ? x(1, W) : x(0, W + 1);
          case m:
            var Q = this.$locale().weekStart || 0, oe = (L < Q ? L + 7 : L) - Q;
            return x(C ? j - oe : j + (6 - oe), W);
          case f:
          case k:
            return H(ne + "Hours", 0);
          case S:
            return H(ne + "Minutes", 1);
          case p:
            return H(ne + "Seconds", 2);
          case v:
            return H(ne + "Milliseconds", 3);
          default:
            return this.clone();
        }
      }, o.endOf = function(a) {
        return this.startOf(a, false);
      }, o.$set = function(a, l) {
        var c, C = D.p(a), w = "set" + (this.$u ? "UTC" : ""), x = (c = {}, c[f] = w + "Date", c[k] = w + "Date", c[g] = w + "Month", c[b] = w + "FullYear", c[S] = w + "Hours", c[p] = w + "Minutes", c[v] = w + "Seconds", c[u] = w + "Milliseconds", c)[C], H = C === f ? this.$D + (l - this.$W) : l;
        if (C === g || C === b) {
          var L = this.clone().set(k, 1);
          L.$d[x](H), L.init(), this.$d = L.set(k, Math.min(this.$D, L.daysInMonth())).$d;
        } else x && this.$d[x](H);
        return this.init(), this;
      }, o.set = function(a, l) {
        return this.clone().$set(a, l);
      }, o.get = function(a) {
        return this[D.p(a)]();
      }, o.add = function(a, l) {
        var c, C = this;
        a = Number(a);
        var w = D.p(l), x = function(W) {
          var j = T(C);
          return D.w(j.date(j.date() + Math.round(W * a)), C);
        };
        if (w === g) return this.set(g, this.$M + a);
        if (w === b) return this.set(b, this.$y + a);
        if (w === f) return x(1);
        if (w === m) return x(7);
        var H = (c = {}, c[p] = n, c[S] = h, c[v] = i, c)[w] || 1, L = this.$d.getTime() + a * H;
        return D.w(L, this);
      }, o.subtract = function(a, l) {
        return this.add(-1 * a, l);
      }, o.format = function(a) {
        var l = this, c = this.$locale();
        if (!this.isValid()) return c.invalidDate || R;
        var C = a || "YYYY-MM-DDTHH:mm:ssZ", w = D.z(this), x = this.$H, H = this.$m, L = this.$M, W = c.weekdays, j = c.months, ne = c.meridiem, Q = function(U, G, ie, ue) {
          return U && (U[G] || U(l, C)) || ie[G].slice(0, ue);
        }, oe = function(U) {
          return D.s(x % 12 || 12, U, "0");
        }, X = ne || function(U, G, ie) {
          var ue = U < 12 ? "AM" : "PM";
          return ie ? ue.toLowerCase() : ue;
        };
        return C.replace(y, function(U, G) {
          return G || (function(ie) {
            switch (ie) {
              case "YY":
                return String(l.$y).slice(-2);
              case "YYYY":
                return D.s(l.$y, 4, "0");
              case "M":
                return L + 1;
              case "MM":
                return D.s(L + 1, 2, "0");
              case "MMM":
                return Q(c.monthsShort, L, j, 3);
              case "MMMM":
                return Q(j, L);
              case "D":
                return l.$D;
              case "DD":
                return D.s(l.$D, 2, "0");
              case "d":
                return String(l.$W);
              case "dd":
                return Q(c.weekdaysMin, l.$W, W, 2);
              case "ddd":
                return Q(c.weekdaysShort, l.$W, W, 3);
              case "dddd":
                return W[l.$W];
              case "H":
                return String(x);
              case "HH":
                return D.s(x, 2, "0");
              case "h":
                return oe(1);
              case "hh":
                return oe(2);
              case "a":
                return X(x, H, true);
              case "A":
                return X(x, H, false);
              case "m":
                return String(H);
              case "mm":
                return D.s(H, 2, "0");
              case "s":
                return String(l.$s);
              case "ss":
                return D.s(l.$s, 2, "0");
              case "SSS":
                return D.s(l.$ms, 3, "0");
              case "Z":
                return w;
            }
            return null;
          })(U) || w.replace(":", "");
        });
      }, o.utcOffset = function() {
        return 15 * -Math.round(this.$d.getTimezoneOffset() / 15);
      }, o.diff = function(a, l, c) {
        var C, w = this, x = D.p(l), H = T(a), L = (H.utcOffset() - this.utcOffset()) * n, W = this - H, j = function() {
          return D.m(w, H);
        };
        switch (x) {
          case b:
            C = j() / 12;
            break;
          case g:
            C = j();
            break;
          case M:
            C = j() / 3;
            break;
          case m:
            C = (W - L) / 6048e5;
            break;
          case f:
            C = (W - L) / 864e5;
            break;
          case S:
            C = W / h;
            break;
          case p:
            C = W / n;
            break;
          case v:
            C = W / i;
            break;
          default:
            C = W;
        }
        return c ? C : D.a(C);
      }, o.daysInMonth = function() {
        return this.endOf(g).$D;
      }, o.$locale = function() {
        return A[this.$L];
      }, o.locale = function(a, l) {
        if (!a) return this.$L;
        var c = this.clone(), C = P(a, l, true);
        return C && (c.$L = C), c;
      }, o.clone = function() {
        return D.w(this.$d, this);
      }, o.toDate = function() {
        return new Date(this.valueOf());
      }, o.toJSON = function() {
        return this.isValid() ? this.toISOString() : null;
      }, o.toISOString = function() {
        return this.$d.toISOString();
      }, o.toString = function() {
        return this.$d.toUTCString();
      }, d;
    })(), le = q.prototype;
    return T.prototype = le, [["$ms", u], ["$s", v], ["$m", p], ["$H", S], ["$W", f], ["$M", g], ["$y", b], ["$D", k]].forEach(function(d) {
      le[d[1]] = function(o) {
        return this.$g(o, d[0], d[1]);
      };
    }), T.extend = function(d, o) {
      return d.$i || (d(o, q, T), d.$i = true), T;
    }, T.locale = P, T.isDayjs = Z, T.unix = function(d) {
      return T(1e3 * d);
    }, T.en = A[N], T.Ls = A, T.p = {}, T;
  });
})(Le);
var Et = Le.exports;
const Mt = Ze(Et);
var ye = function(s) {
  if (!s) return 0;
  var t = Mt(s).valueOf() - Date.now();
  return t < 0 ? 0 : t;
}, Dt = function(s) {
  return { days: Math.floor(s / 864e5), hours: Math.floor(s / 36e5) % 24, minutes: Math.floor(s / 6e4) % 60, seconds: Math.floor(s / 1e3) % 60, milliseconds: Math.floor(s) % 1e3 };
}, $t = function(s) {
  s === void 0 && (s = {});
  var t = s || {}, i = t.leftTime, n = t.targetDate, h = t.interval, u = h === void 0 ? 1e3 : h, v = t.onEnd, p = r.useMemo(function() {
    return qe(i) && i > 0 ? Date.now() + i : void 0;
  }, [i]), S = "leftTime" in s ? p : n, f = ze(r.useState(function() {
    return ye(S);
  }), 2), m = f[0], g = f[1], M = Ge(v);
  r.useEffect(function() {
    if (!S) {
      g(0);
      return;
    }
    g(ye(S));
    var k = setInterval(function() {
      var R, I = ye(S);
      g(I), I === 0 && (clearInterval(k), (R = M.current) === null || R === void 0 || R.call(M));
    }, u);
    return function() {
      return clearInterval(k);
    };
  }, [S, u]);
  var b = r.useMemo(function() {
    return Dt(m);
  }, [m]);
  return [m, b];
};
const Rt = $t, Ve = (s) => {
  const { className: t, title: i, subtitle: n, onTransitionEnd: h, innerRef: u } = s, { classes: v } = Tt();
  return E("div", { className: K(t, v.root), onTransitionEnd: h, ref: u, children: [e(_, { top: 40, children: e(V, { variant: "h2", color: "primary", children: i }) }), n && e(_, { top: 30, children: e(V, { color: "primary", children: n }) })] });
}, Tt = z()({ root: { display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center" } });
Ve.defaultProps = { className: "", innerRef: null, onTransitionEnd: () => {
} };
const ce = (s) => {
  const { animationProps: t, actions: i, image: n, disableFadeIn: h, isHidden: u, isCenteredByHeight: v, title: p, children: S, subtitle: f } = s, { classes: m } = Pt(), g = n ? e("div", { className: m.imageContainer, children: n }) : null;
  return e("div", { className: K(m.root, { [m.fadeIn]: !h, [m.hidden]: u }), children: e(Ne, { fixedFooter: true, hideOverflow: t.isAnimationInProgress, classes: { center: K(m.center, { [m.centeredByHeight]: v }) }, top: e(ee, { order: 1, animationProps: t, children: e("div", { className: m.header, children: e(Ve, { title: p, subtitle: f }) }) }), center: e(ee, { isLastElement: true, order: 2, animationProps: t, children: S ? E("div", { children: [S, g] }) : g || e("div", {}) }), bottom: e(kt, { children: i }) }) });
}, xt = he`
  0% {
    opacity: 1;
  }
  100% {
    opacity: 0;
  }
`, Pt = z()({ ...Ce, root: { height: "100%", width: "100%", backgroundColor: "var(--dark-grey-1)", position: "absolute", top: "0", left: "0", right: "0", bottom: "0" }, center: { height: "auto", minHeight: "auto", alignItems: "flex-start" }, centeredByHeight: { alignItems: "center", overflow: "hidden" }, header: { width: "100%", maxWidth: "400px", margin: "auto", marginBottom: "40px" }, hidden: { animation: xt, animationDelay: "0.5s", animationFillMode: "forwards" }, imageContainer: { maxWidth: "600px", width: "100%", height: "80%", display: "flex", alignItems: "center", justifyContent: "center" } });
ce.defaultProps = { actions: null, disableFadeIn: false, isCenteredByHeight: false, isHidden: false, subtitle: "", children: null };
const It = (s) => {
  const { onClose: t, animationProps: i, isHidden: n } = s;
  return e(ce, { isHidden: n, animationProps: i, title: "Already enabled", subtitle: E(se.Fragment, { children: [e(_, { bottom: 15, children: "Looks like the new encryption is already enabled for your account! 🎉" }), "Your data is now protected by our new data encryption algorithm. Learn more about new encryption", " ", e(me, { color: "var(--accent)", underline: true, onClick: we, children: "here" }), ".", e(_, { top: 15, children: "No further action is required." })] }), actions: e(B, { fullWidth: true, color: "accent", onClick: t, children: "Done" }) });
}, Se = (s) => {
  const { animationProps: t, children: i, actions: n, image: h, disableFadeIn: u, isHidden: v } = s, { classes: p } = _t();
  return e("div", { className: K({ [p.fadeIn]: !u, [p.hidden]: v }), children: e(ee, { animationProps: t, isContainer: true, children: e("div", { className: K(p.root, { "termius-dark-theme": true }), children: e(Ne, { top: e(ee, { order: 1, animationProps: t, children: e(V, { color: "primary", align: "center", variant: "h2", children: "We're improving your data security" }) }), left: e(ee, { order: 2, animationProps: t, children: e("div", { className: p.imageContainer, children: h }) }), right: e(ee, { order: 2, animationProps: t, children: e("div", { className: p.cardContainer, children: E("div", { children: [e(Je, { className: p.card, dark: true, children: i }), e(_, { top: 20, children: n })] }) }) }) }) }) }) });
}, Ht = he`
  0% {
    opacity: 1;
  }
  100% {
    opacity: 0;
  }`, _t = z()({ ...Ce, root: { height: "100%", width: "100%", padding: "40px", paddingBottom: "0", backgroundColor: "var(--dark-grey-1)", position: "absolute", top: "0", left: "0", right: "0", bottom: "0" }, hidden: { animation: Ht, animationDelay: "0.5s", animationFillMode: "forwards" }, imageContainer: { width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center", "@media only screen and (max-width: 900px)": { display: "none" } }, cardContainer: { display: "flex", flexDirection: "column", justifyContent: "center", alignItems: "center", position: "relative" }, card: { width: "440px", maxWidth: "100%" } });
Se.defaultProps = { actions: null, disableFadeIn: false, isHidden: false };
var Ot = ["years", "months", "weeks", "days", "hours", "minutes", "seconds"];
function Nt(s, t) {
  var i, n, h, u, v;
  if (arguments.length < 1) throw new TypeError("1 argument required, but only ".concat(arguments.length, " present"));
  var p = Qe(), S = (i = (n = t == null ? void 0 : t.locale) !== null && n !== void 0 ? n : p.locale) !== null && i !== void 0 ? i : Ke, f = (h = t == null ? void 0 : t.format) !== null && h !== void 0 ? h : Ot, m = (u = t == null ? void 0 : t.zero) !== null && u !== void 0 ? u : false, g = (v = t == null ? void 0 : t.delimiter) !== null && v !== void 0 ? v : " ";
  if (!S.formatDistance) return "";
  var M = f.reduce(function(b, k) {
    var R = "x".concat(k.replace(/(^.)/, function(y) {
      return y.toUpperCase();
    })), I = s[k];
    return typeof I == "number" && (m || s[k]) ? b.concat(S.formatDistance(R, I)) : b;
  }, []).join(g);
  return M;
}
const ve = (s = "") => Number(s.replace(/\D+/g, "")) || 45, We = (s) => `Too many failed attempts. Try again after ${Nt({ seconds: s }, { format: ["minutes", "seconds"] })}`;
function Ft() {
  const [s, t] = r.useState(null), i = r.useRef(), n = (h) => {
    i.current || (i.current = setInterval(() => {
      t((u) => u == null ? h : u < 1 ? (clearInterval(i.current), i.current = null, null) : u - 1);
    }, 1e3));
  };
  return r.useEffect(() => () => clearInterval(i.current), []), [s, n];
}
const At = (s) => {
  const { onOk: t } = s, [i, n] = r.useState(null), [h, u] = r.useState(""), [v, p] = Ft(), { register: S, handleSubmit: f, watch: m } = Pe();
  r.useEffect(() => {
    if (!v) {
      n(null);
      return;
    }
    u(We(v)), n("error");
  }, [v]);
  const g = r.useCallback(async ({ password: k }) => {
    var R, I, y, $, F;
    try {
      u("Enabling new crypto"), n("loading"), await t({ password: k });
    } catch (O) {
      if (O.status === 0) {
        u("Network request was timed out. Please check your connection."), n("error");
        return;
      }
      if (O.status === 429) {
        const N = ve((R = O.responseJSON) == null ? void 0 : R.detail);
        p(N);
        return;
      }
      if ((I = O.responseJSON) != null && I.password) {
        u((y = O.responseJSON.password) == null ? void 0 : y[0]), n("error");
        return;
      }
      u(O.message || O.detail || (($ = O.responseJSON) == null ? void 0 : $.message) || ((F = O.responseJSON) == null ? void 0 : F.detail) || "Oops, something went wrong. Please try again later."), n("error");
    }
  }, [t, p]), M = m("password"), b = r.useMemo(() => i === "loading" || !M, [M, i]);
  return E("form", { onSubmit: f(g), children: [E(V, { color: "primary", variant: "h2", children: ["Enter ", te] }), e(_, { top: 20, children: E(V, { color: "primary", fontSize: 14, children: ["Please enter your ", te, " below to migrate your data to the new, more secure encryption algorithm."] }) }), e(_, { top: 10, bottom: 20, children: e(Ae, { autoFocus: true, InputProps: S("password"), disabled: i === "loading", onChange: () => n(null), label: `${Ie(te)}`, fullWidth: true }) }), i && e(_, { bottom: 20, children: e(re, { status: i, children: h }) }), e(fe, { children: e(B, { color: "accent", disabled: b, type: "submit", fullWidth: true, children: "Enable new encryption" }) })] });
};
function Lt(s) {
  Xe(1, arguments);
  var t = s / et;
  return Math.floor(t);
}
function Vt(s) {
  const { classes: t } = jt(), { onOk: i } = s, { register: n, watch: h, formState: u, handleSubmit: v } = Pe({ defaultValues: { code: "", newPassword: "" } }), p = h(), { isPasswordValidForSignUp: S, ...f } = tt(p.newPassword), { statuses: m, countdownInSeconds: g, submitDisabled: M, codeRequest: b, migrationRequest: k } = Yt({ onOk: i, formState: u, isPasswordValidForSignUp: S });
  return E("form", { onSubmit: v(k.run), children: [E(V, { color: "primary", variant: "h2", children: ["Forgot your ", te, "?"] }), e(_, { top: 20, children: E(V, { color: "primary", fontSize: 14, children: ["We need to make sure that it's still you. Enter the code from your email and your new ", te, " in the fields below."] }) }), E(_, { top: 10, bottom: 20, children: [e(rt, { className: t == null ? void 0 : t.verificationCodeField, InputProps: { ...n("code", { required: true }), disableUnderline: true, classes: { input: t == null ? void 0 : t.verificationCodeInput } }, placeholder: "CODE", error: m.code.status === "error", fullWidth: true, autoFocus: true }), e(Wt, { pb: "20px", status: m.code, countdown: g, onRetry: b.refresh }), e(_, { top: 10, children: e(Ae, { label: `New ${Ie(te)}`, error: u.errors.newPassword !== void 0, fullWidth: true, InputProps: { ...n("newPassword", { required: true }) } }) }), e(_, { top: 20, children: e(nt, { ...f }) }), e(Bt, { pb: "20px", status: m.migration, countdown: g })] }), e(fe, { children: e(B, { color: "accent", disabled: M, type: "submit", fullWidth: true, children: "Enable new encryption" }) })] });
}
function $e(s) {
  const { countdown: t, retryButtonText: i, onRetry: n } = s;
  return t <= 0 ? e(me, { underline: true, onClick: n, children: i }) : E("span", { children: ["Resend available in ", t, " seconds."] });
}
function Wt(s) {
  const { status: t, countdown: i, onRetry: n, ...h } = s;
  if (t.status === "idle") return null;
  if (t.status === "loading") return e(ae, { ...h, children: e(re, { status: t.status, children: "Sending the code..." }) });
  const u = t.status === "error" ? "Retry" : "Resend code?";
  return t.status === "error" ? e(ae, { ...h, children: E(re, { status: t.status, children: [t.message, " ", e($e, { countdown: i, onRetry: n, retryButtonText: u })] }) }) : e(ae, { ...h, children: E(re, { status: t.status, children: ["We've sent the code to your email.", " ", e($e, { countdown: i, onRetry: n, retryButtonText: "Resend code?" })] }) });
}
function Bt(s) {
  const { status: t, countdown: i, ...n } = s;
  return t.status === "loading" ? e(ae, { ...n, children: e(re, { status: t.status, children: "Enabling new crypto" }) }) : t.status === "error" && i > 0 ? e(ae, { ...n, children: e(re, { status: t.status, children: We(i) }) }) : t.status === "error" ? e(ae, { ...n, children: e(re, { status: t.status, children: t.message }) }) : null;
}
const Re = "Network request was timed out. Please check your connection.", Te = "Oops, something went wrong. Please try again later.", Ut = (s) => s.replace("Security token", "The verification code").replace("security token", "the verification code"), jt = z()({ verificationCodeField: { borderRadius: "10px", background: "var(--dark-grey-4)" }, verificationCodeInput: { fontSize: "32px", textAlign: "center", height: "59px" } });
function Yt(s) {
  const { onOk: t, formState: i, isPasswordValidForSignUp: n } = s, h = pe(), [u, v] = r.useState(), [p] = Rt({ targetDate: u }), S = Lt(p), f = (y) => {
    const F = Date.now() + y;
    v(F);
  }, [m, g] = r.useState({ code: { status: "idle", message: void 0 }, migration: { status: "idle", message: void 0 } }), M = (y) => {
    g(ke(($) => {
      $.code.status = y.status ?? $.code.status, $.code.message = y.message ?? $.code.message;
    }));
  }, b = (y) => {
    g(ke(($) => {
      $.migration.status = y.status ?? $.migration.status, $.migration.message = y.message ?? $.migration.message;
    }));
  }, k = De(async () => J(h, at), { onBefore: () => {
    M({ status: "loading", message: void 0 });
  }, onSuccess: (y) => {
    const $ = be(y.now), F = be(y.next_retry_available_at);
    f(st(F, $)), M({ status: "success", message: void 0 });
  }, onError: (y) => {
    if (y.status === 0) {
      M({ status: "error", message: Re });
      return;
    }
    if (y.status === 429) {
      f(ve(y.detail) * 1e3), M({ status: "error", message: void 0 });
      return;
    }
    M({ status: "error", message: y.detail || Te });
  } }), R = De(t, { manual: true, onBefore: () => {
    b({ status: "loading", message: void 0 }), M({ status: "idle", message: void 0 });
  }, onSuccess: () => {
    b({ status: "success", message: void 0 });
  }, onError: (y) => {
    var O, N, A, Y, Z, P;
    if (y.status === 0) {
      b({ status: "error", message: Re });
      return;
    }
    if (y.status === 429) {
      f(ve((O = y.responseJSON) == null ? void 0 : O.detail) * 1e3), b({ status: "error", message: void 0 });
      return;
    }
    const $ = Ut(((A = (N = y.responseJSON) == null ? void 0 : N.security_token) == null ? void 0 : A[0]) ?? "");
    if (y.status === 400 && ((Y = y.responseJSON) != null && Y.security_token)) {
      M({ status: "error", message: $ }), b({ status: "idle", message: void 0 });
      return;
    }
    const F = ((Z = y.responseJSON) == null ? void 0 : Z.message) || ((P = y.responseJSON) == null ? void 0 : P.detail) || "";
    b({ status: "error", message: $ || F || Te });
  } }), I = !!(m.code.status === "loading" || m.migration.status === "loading" || i.errors.code || i.errors.newPassword || !n);
  return { statuses: m, countdownInSeconds: S, submitDisabled: I, codeRequest: k, migrationRequest: R };
}
const Zt = (s) => {
  const { onContinue: t, onClose: i, onResetPassword: n, isControlsDisabled: h, animationProps: u, isHidden: v, onAlreadyMigrated: p } = s, S = pe(), f = ge(), [m, g] = r.useState(false), M = He(), b = r.useCallback(async () => {
    const y = await J(S, _e), { encryption_schema: $ } = Oe(y);
    return $ === "v5";
  }, [S]), k = r.useCallback(async (y) => await M(b()) ? (p(), null) : n(y), [M, b, p, n]), R = r.useCallback(async (y) => await M(b()) ? (p(), null) : t(y), [M, b, p, t]), I = r.useCallback(() => {
    g(true), f(de("ForgotPassword"));
  }, [f]);
  return r.useEffect(() => {
    f(de("EnterPassword"));
  }, []), e(Se, { isHidden: v, animationProps: u, image: e(Fe, {}), actions: E(fe, { children: [e(B, { disabled: h, fullWidth: true, onClick: i, children: "Do it later" }), m ? e("div", { style: { width: "100%" } }) : E(B, { disabled: h, fullWidth: true, onClick: I, children: ["Forgot ", te] })] }), children: m ? e(Vt, { onOk: k }) : e(At, { onOk: R }) });
}, Be = (s) => {
  const { className: t, src: i, alt: n, onTransitionEnd: h, innerRef: u } = s, { classes: v } = qt();
  return e("img", { className: K(t, v.picture), src: i, alt: n, onTransitionEnd: h, ref: u });
};
Be.defaultProps = { className: "", innerRef: null, onTransitionEnd: () => {
} };
const qt = z()({ picture: { maxHeight: "450px", maxWidth: "90%", height: "100%", objectFit: "contain", WebkitUserDrag: "none" } }), zt = "" + new URL("migrationIntro-bd2f03dd.svg", import.meta.url).href, Gt = (s) => {
  const { onContinue: t, onClose: i, animationProps: n, isHidden: h } = s;
  return e(ce, { disableFadeIn: true, isHidden: h, animationProps: n, title: "New Encryption", subtitle: E(V, { color: "primary", fontSize: 14, children: ["We've updated our data encryption algorithm to make it even more secure. Learn more about new encryption", " ", e(me, { color: "var(--accent)", underline: true, onClick: we, children: "here" }), ".", e("br", {}), e("br", {}), "You can continue working and sync your data without migration. You will be able to do it later, but we recommend you do it now to protect your data."] }), image: e(Be, { src: zt, alt: "migration" }), actions: E(se.Fragment, { children: [e(B, { onClick: t, color: "accent", type: "button", fullWidth: true, children: "Enable new encryption" }), e(_, { top: 20, children: e(B, { fullWidth: true, onClick: i, children: "Do it later" }) })] }) });
}, Jt = (s, t) => r.createElement("svg", { width: 239, height: 226, viewBox: "0 0 239 226", fill: "none", xmlns: "http://www.w3.org/2000/svg", ref: t, ...s }, r.createElement("g", { filter: "url(#dark_error_filter0_f)" }, r.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M76.7402 179.602H161.195V187.772H76.7402V179.602Z", fill: "#1B3554" })), r.createElement("path", { opacity: 0.341882, fillRule: "evenodd", clipRule: "evenodd", d: "M115.519 224.506C174.358 224.506 222.057 176.807 222.057 117.967C222.057 59.1281 174.358 11.4294 115.519 11.4294C56.6792 11.4294 8.98047 59.1281 8.98047 117.967C8.98047 176.807 56.6792 224.506 115.519 224.506Z", stroke: "white", strokeWidth: 1.47449, strokeLinecap: "round", strokeLinejoin: "round", strokeDasharray: "0.74 8.85" }), r.createElement("g", { filter: "url(#dark_error_filter1_d)" }, r.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M62.0449 82.0591C62.0449 80.0233 63.6953 78.3729 65.7311 78.3729H166.121C168.157 78.3729 169.808 80.0233 169.808 82.0591V153.06C169.808 155.095 168.157 156.746 166.121 156.746H65.7311C63.6953 156.746 62.0449 155.095 62.0449 153.06V82.0591Z", fill: "#32364A" })), r.createElement("mask", { id: "dark_error_mask0", "mask-type": "alpha", maskUnits: "userSpaceOnUse", x: 62, y: 78, width: 108, height: 79 }, r.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M62.0449 82.0591C62.0449 80.0233 63.6953 78.3729 65.7311 78.3729H166.121C168.157 78.3729 169.808 80.0233 169.808 82.0591V153.06C169.808 155.095 168.157 156.746 166.121 156.746H65.7311C63.6953 156.746 62.0449 155.095 62.0449 153.06V82.0591Z", fill: "white" })), r.createElement("g", { mask: "url(#dark_error_mask0)" }, r.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M68.0273 87.7357C68.0273 87.3285 68.3574 86.9984 68.7646 86.9984H163.282C163.689 86.9984 164.019 87.3285 164.019 87.7357V147.048C164.019 147.456 163.689 147.786 163.282 147.786H68.7646C68.3574 147.786 68.0273 147.456 68.0273 147.048L68.0273 87.7357Z", fill: "#141729" })), r.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M222.267 109.714C230.95 109.714 237.989 102.698 237.989 94.0436C237.989 85.3889 230.95 78.3729 222.267 78.3729C213.584 78.3729 206.545 85.3889 206.545 94.0436C206.545 102.698 213.584 109.714 222.267 109.714Z", fill: "#F25E61" }), r.createElement("path", { d: "M224.153 87.3052H220.799L221.444 96.2812H223.508L224.153 87.3052ZM220.873 98.9721C220.873 99.8384 221.573 100.557 222.458 100.557C223.342 100.557 224.061 99.8384 224.061 98.9721C224.061 98.0874 223.342 97.3686 222.458 97.3686C221.573 97.3686 220.873 98.0874 220.873 98.9721Z", fill: "white" }), r.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M167.359 225.322C174.573 225.322 180.421 219.657 180.421 212.668C180.421 205.679 174.573 200.014 167.359 200.014C160.145 200.014 154.297 205.679 154.297 212.668C154.297 219.657 160.145 225.322 167.359 225.322Z", fill: "#F25E61" }), r.createElement("path", { d: "M168.71 207.56L168.194 214.741H166.543L166.027 207.56H168.71ZM166.086 216.894C166.086 216.54 166.208 216.24 166.454 215.994C166.7 215.739 167 215.611 167.354 215.611C167.708 215.611 168.007 215.739 168.253 215.994C168.509 216.24 168.636 216.54 168.636 216.894C168.636 217.238 168.509 217.537 168.253 217.793C168.007 218.039 167.708 218.162 167.354 218.162C167 218.162 166.7 218.039 166.454 217.793C166.208 217.537 166.086 217.238 166.086 216.894Z", fill: "white" }), r.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M84.6915 38.6357C95.3953 38.6357 104.072 29.9868 104.072 19.3178C104.072 8.6489 95.3953 0 84.6915 0C73.9877 0 65.3105 8.6489 65.3105 19.3178C65.3105 29.9868 73.9877 38.6357 84.6915 38.6357Z", fill: "#F25E61" }), r.createElement("path", { fillRule: "evenodd", clipRule: "evenodd", d: "M19.7531 181.428C30.6625 181.428 39.5063 172.613 39.5063 161.74C39.5063 150.866 30.6625 142.051 19.7531 142.051C8.84378 142.051 0 150.866 0 161.74C0 172.613 8.84378 181.428 19.7531 181.428Z", fill: "#F25E61" }), r.createElement("path", { d: "M21.62 154.37L20.8459 165.141H18.3687L17.5946 154.37H21.62ZM17.6831 168.37C17.6831 167.839 17.8674 167.389 18.236 167.021C18.6046 166.637 19.0544 166.446 19.5852 166.446C20.116 166.446 20.5657 166.637 20.9343 167.021C21.3177 167.389 21.5094 167.839 21.5094 168.37C21.5094 168.886 21.3177 169.336 20.9343 169.719C20.5657 170.088 20.116 170.272 19.5852 170.272C19.0544 170.272 18.6046 170.088 18.236 169.719C17.8674 169.336 17.6831 168.886 17.6831 168.37Z", fill: "white" }), r.createElement("path", { d: "M77.2193 99.9859V99.1292H73.7513V96.7656H77.1057V95.9089H73.7513V93.6382H77.2193V92.7815H72.7501V99.9859H77.2193ZM78.8151 99.9859H83.2842V99.1911H81.2716V97.0236C81.2716 96.2392 81.7876 95.5993 82.7785 95.5993H83.8106V94.66H83.1088C82.0044 94.66 81.5296 95.3206 81.3335 96.0328H81.2716V94.66H78.8151V95.4548H80.3117V99.1911H78.8151V99.9859ZM85.0039 99.9859H89.4731V99.1911H87.4604V97.0236C87.4604 96.2392 87.9764 95.5993 88.9673 95.5993H89.9994V94.66H89.2976C88.1932 94.66 87.7184 95.3206 87.5223 96.0328H87.4604V94.66H85.0039V95.4548H86.5005V99.1911H85.0039V99.9859ZM93.5047 100.11C95.0323 100.11 95.9715 99.026 95.9715 97.3229C95.9715 95.6199 95.0323 94.5362 93.5047 94.5362C91.9771 94.5362 91.0379 95.6199 91.0379 97.3229C91.0379 99.026 91.9771 100.11 93.5047 100.11ZM93.5047 99.315C92.6274 99.315 92.0597 98.7886 92.0597 97.7152V96.9307C92.0597 95.8573 92.6274 95.3309 93.5047 95.3309C94.382 95.3309 94.9497 95.8573 94.9497 96.9307V97.7152C94.9497 98.7886 94.382 99.315 93.5047 99.315ZM97.3815 99.9859H101.851V99.1911H99.838V97.0236C99.838 96.2392 100.354 95.5993 101.345 95.5993H102.377V94.66H101.675C100.571 94.66 100.096 95.3206 99.9 96.0328H99.838V94.66H97.3815V95.4548H98.8781V99.1911H97.3815V99.9859Z", fill: "#F25E61" }), r.createElement("path", { d: "M86.5223 11.3188L85.7482 22.0899H83.2711L82.497 11.3188H86.5223ZM82.5854 25.3191C82.5854 24.7883 82.7698 24.3385 83.1384 23.9699C83.507 23.5866 83.9567 23.3949 84.4875 23.3949C85.0184 23.3949 85.4681 23.5866 85.8367 23.9699C86.2201 24.3385 86.4117 24.7883 86.4117 25.3191C86.4117 25.8352 86.2201 26.2849 85.8367 26.6682C85.4681 27.0369 85.0184 27.2212 84.4875 27.2212C83.9567 27.2212 83.507 27.0369 83.1384 26.6682C82.7698 26.2849 82.5854 25.8352 82.5854 25.3191Z", fill: "white" }), r.createElement("rect", { opacity: 0.8, x: 107.763, y: 130.619, width: 4.8983, height: 10.613, rx: 1.47449, fill: "#F25E61" }), r.createElement("defs", null, r.createElement("filter", { id: "dark_error_filter0_f", x: 55.233, y: 158.094, width: 127.469, height: 51.1848, filterUnits: "userSpaceOnUse", colorInterpolationFilters: "sRGB" }, r.createElement("feFlood", { floodOpacity: 0, result: "BackgroundImageFix" }), r.createElement("feBlend", { mode: "normal", in: "SourceGraphic", in2: "BackgroundImageFix", result: "shape" }), r.createElement("feGaussianBlur", { stdDeviation: 10.7536, result: "effect1_foregroundBlur" })), r.createElement("filter", { id: "dark_error_filter1_d", x: 54.6725, y: 68.0515, width: 122.508, height: 93.1178, filterUnits: "userSpaceOnUse", colorInterpolationFilters: "sRGB" }, r.createElement("feFlood", { floodOpacity: 0, result: "BackgroundImageFix" }), r.createElement("feColorMatrix", { in: "SourceAlpha", type: "matrix", values: "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 127 0" }), r.createElement("feOffset", { dy: -2.94898 }), r.createElement("feGaussianBlur", { stdDeviation: 3.68622 }), r.createElement("feColorMatrix", { type: "matrix", values: "0 0 0 0 0.191214 0 0 0 0 0.288182 0 0 0 0 0.613202 0 0 0 0.154552 0" }), r.createElement("feBlend", { mode: "normal", in2: "BackgroundImageFix", result: "effect1_dropShadow" }), r.createElement("feBlend", { mode: "normal", in: "SourceGraphic", in2: "effect1_dropShadow", result: "shape" })))), Kt = r.forwardRef(Jt), Qt = r.memo(Kt), Ue = (s) => {
  const { className: t, progressCircleClassName: i, backgroundCircleClassName: n, children: h, size: u = 120, isSuccess: v = false, ...p } = s, { cx: S, classes: f } = tr({ size: u });
  return E("div", { className: S(f.root, t), children: [h, v && e("div", { className: f.check, children: e(ot, { width: "112px", height: "112px", className: f.checkImage }) }), e(Ee, { ...p, size: u, value: 100, className: S(f.progressBackground, n), variant: "determinate" }), e(Ee, { ...p, size: u, value: 20, className: S(f.progress, i), variant: "determinate", classes: { circle: S({ [f.success]: v }) } })] });
}, Xt = he`
  0% {
    transform: rotate(-100deg);
  }
  100% {
    transform: rotate(260deg);
  }
`, er = he`
  to {
    stroke-dashoffset: 0;
  }
`, tr = z()((s, { size: t }) => ({ root: { position: "relative", margin: "auto", width: t, height: t }, check: { position: "absolute", width: "100%", height: "100%", display: "flex", alignItems: "center", justifyContent: "center" }, checkImage: { fill: "var(--green)" }, progress: { color: "var(--green)", position: "absolute", animation: `${Xt} 0.8s linear infinite` }, progressBackground: { color: "var(--light-grey-4)", position: "absolute", ".termius-dark-theme &": { color: "var(--dark-grey-4)" } }, success: { animation: `${er} 0.5s`, animationFillMode: "both" } }));
Ue.defaultProps = { thickness: 3 };
const rr = "" + new URL("background_circle-1452d38b.svg", import.meta.url).href, nr = (s) => {
  const { classes: t } = sr(), i = it(ct), n = pe(), [h, u] = r.useState("check"), v = r.useRef(null), p = He(), { onContinue: S, onClose: f, animationProps: m, isHidden: g, onUpdateDevices: M, onAlreadyMigrated: b } = s;
  r.useEffect(() => {
    m.isAnimationInProgress && (clearTimeout(v.current), v.current = null);
  }, [m.isAnimationInProgress]);
  const k = r.useCallback(() => {
    u("success"), v.current = setTimeout(() => S(), 1e3);
  }, [S]), R = r.useCallback(() => J(n, lt), [n]), I = r.useCallback(() => J(n, ut), [n]), y = r.useCallback((P, T = []) => {
    T.length || P.length ? M({ members: T, devices: P }) : k();
  }, [k, M]), $ = r.useCallback(async () => {
    const P = await J(n, _e), { encryption_schema: T } = Oe(P);
    return T === "v5";
  }, [n]), F = r.useCallback(async () => {
    u("check");
    try {
      if (await p($())) {
        b();
        return;
      }
      await p(J(n, dt, "does-not-block-ui"));
      const { objects: P } = await p(R());
      if (i) {
        const { objects: T } = await p(I());
        y(P, T);
        return;
      }
      y(P);
    } catch (P) {
      if (P instanceof ht) return;
      if (P === 0 || (P == null ? void 0 : P.status) === 0) {
        u("offline");
        return;
      }
      u("error");
    }
  }, [p, $, n, R, i, y, b, I]), O = e(Ue, { isSuccess: h === "success", size: 205, thickness: 2 }), N = { check: null, offline: e(mt, { className: t.offlineImage }), error: e(Qt, {}), success: null }, A = { check: "Checking your setup...", offline: "Offline", error: E(se.Fragment, { children: ["Oops ", e("span", { role: "img", children: "🤷‍♀️" })] }), success: "Checking your setup..." }, Y = { check: null, offline: "Please make sure that your internet connection is active to proceed.", error: "Looks like there was an error with your request. Please try again.", success: null }, Z = { check: e(B, { fullWidth: true, onClick: f, children: "Do it later" }), offline: E(se.Fragment, { children: [e(B, { onClick: F, color: "accent", type: "button", fullWidth: true, children: "Retry" }), e(_, { top: 20, children: e(B, { fullWidth: true, onClick: f, children: "Do it later" }) })] }), error: E(se.Fragment, { children: [e(B, { onClick: F, color: "accent", type: "button", fullWidth: true, children: "Retry" }), e(_, { top: 20, children: e(B, { fullWidth: true, onClick: f, children: "Do it later" }) })] }), success: e(B, { fullWidth: true, onClick: f, children: "Do it later" }) };
  return r.useEffect(() => {
    F();
  }, []), E(ce, { isCenteredByHeight: true, isHidden: g, animationProps: m, title: A[h], subtitle: Y[h], image: e("div", { className: t.image }), actions: Z[h], children: [["check", "success"].includes(h) && O, N[h]] });
}, sr = z()({ image: { position: "absolute", left: "50%", top: "50%", backgroundImage: `url(${rr})`, backgroundRepeat: "no-repeat", backgroundPosition: "top", transform: "translate(-50%, 0)", width: "100vw", height: "50vh", zIndex: -1 }, offlineImage: { color: "var(--white)", width: "150px", height: "150px" } }), ar = (s) => {
  const { onClose: t, animationProps: i } = s, n = ge();
  return r.useEffect(() => {
    n(de("Success"));
  }, []), E(Se, { animationProps: i, image: e(Fe, {}), children: [e(V, { color: "primary", variant: "h2", children: "Migration was successful!" }), e(_, { top: 20, bottom: 20, children: E(V, { color: "primary", fontSize: 14, children: ["Congratulations!", " ", E("span", { role: "img", "aria-label": ":tada:", children: ["🎉", " "] }), " ", "Your data is now protected by our new data encryption algorithm. Learn more about new encryption", " ", e(me, { color: "var(--accent)", underline: true, onClick: we, children: "here" }), ".", e("br", {}), e("br", {}), "Please make sure to update Termius to the latest version on all of your devices."] }) }), e(fe, { children: e(B, { onClick: t, color: "accent", type: "button", fullWidth: true, children: "Done" }) })] });
};
function xe({ className: s, children: t, endAdornment: i = null }) {
  const { classes: n } = or();
  return E("div", { className: K(s, n.root, { [n.withEndAdornment]: !!i }), children: [t, i] });
}
const or = z()({ root: { background: "var(--light-grey-6)", borderRadius: "14px", padding: "12px", display: "block", ".termius-dark-theme &": { background: "var(--dark-grey-4)" } }, withEndAdornment: { display: "flex", justifyContent: "space-between" } }), je = (s) => {
  const { classes: t } = ir(), i = ge(), { onContinue: n, onClose: h, animationProps: u, isHidden: v, members: p } = s, S = ft(s.devices, (m) => m.latest_activity).reverse(), f = pt(S, 10);
  return r.useEffect(() => {
    i(de("BeforeYouProceed"));
  }, []), e(ce, { isHidden: v, animationProps: u, title: "Before you proceed", actions: E(se.Fragment, { children: [e(B, { onClick: n, color: "accent", type: "button", fullWidth: true, children: "I will update" }), e(_, { top: 20, children: e(B, { fullWidth: true, onClick: h, children: "Do it later" }) })] }), children: E("div", { className: t.stepContent, children: [e(V, { color: "primary", fontSize: 14, children: "After enabling the new encryption, latest Termius version will be required." }), !!f.length && E(_, { top: 20, children: [E(V, { color: "primary", fontSize: 14, children: ["Please make sure you update the Termius app to the latest version", " ", e(V, { display: "inline", bold: true, children: "on all devices" }), ":"] }), f.map((m) => e(_, { top: 20, children: e(xe, { children: e(Me, { title: e(V, { lineHeight: 2.5, color: "primary", fontSize: 15, bold: true, children: m.name }) }) }) }, m.id))] }), !!p.length && E(_, { top: 20, children: [E(V, { color: "primary", fontSize: 14, children: ["Please make sure", " ", e(V, { display: "inline", bold: true, children: "all members of your team" }), " ", "update Termius app to the latest version:"] }), p.map((m, g) => e(_, { top: 20, children: e(xe, { children: e(Me, { icon: e(gt, { className: t.identityIcon, icon: "identity" }), title: e(V, { color: "primary", fontSize: 15, bold: true, children: m.email }) }) }) }, `${m.email}-${g.toString()}`))] })] }) });
};
je.defaultProps = { members: [], devices: [] };
const ir = z()({ stepContent: { width: "100%", maxWidth: "400px", marginBottom: "20px" }, identityIcon: { color: "var(--dark-grey-3)", ".termius-dark-theme &": { color: "var(--dark-grey-6)" } } }), cr = ({ mounted: s, opacity: t, translate: i, opacityContainer: n, onTransitionEndContainer: h, onTransitionEndContent: u, contentRef: v, containerRef: p, isAnimationInProgress: S, closeDialog: f, inAppMessage: m }) => {
  const g = r.useMemo(() => ({ mounted: s, opacity: t, translate: i, opacityContainer: n, onTransitionEndContainer: h, onTransitionEndContent: u, contentRef: v, isAnimationInProgress: S }), [s, t, i, n, h, u, v, S]), M = pe(), b = ge(), [k, R] = r.useState("INTRO"), [I, y] = r.useState([]), [$, F] = r.useState([]), [O, N] = r.useState(false), A = r.useCallback(() => {
    b(vt.request(m.id)), f();
  }, [f, b, m]), Y = r.useCallback(() => {
    b(Ct.request([m.id])), f();
  }, [f, b, m]), Z = r.useCallback(async (d) => {
    try {
      N(true), await J(M, wt, d), N(false), R("SUCCESS");
    } catch (o) {
      throw N(false), o;
    }
  }, [R, M]), P = r.useCallback(async (d) => {
    try {
      N(true), await J(M, St, d), N(false), R("SUCCESS");
    } catch (o) {
      throw N(false), o;
    }
  }, [R, M]), T = r.useCallback(({ members: d, devices: o }) => {
    y(d), F(o), R("UPDATE_DEVICES");
  }, []), D = () => {
    R("ALREADY_MIGRATED");
  }, q = r.useMemo(() => ({ INTRO: e(Gt, { animationProps: g, onClose: A, isHidden: k !== "INTRO", onContinue: () => R("SETUP") }), SETUP: e(nr, { animationProps: g, onClose: A, isHidden: k !== "SETUP", onUpdateDevices: T, onAlreadyMigrated: D, onContinue: () => R("ENTER_PASSWORD") }), UPDATE_DEVICES: e(je, { animationProps: g, onClose: A, devices: $, members: I, isHidden: k !== "UPDATE_DEVICES", onContinue: () => R("ENTER_PASSWORD") }), ENTER_PASSWORD: e(Zt, { animationProps: g, onResetPassword: P, onContinue: Z, onAlreadyMigrated: D, isControlsDisabled: O, isHidden: k !== "ENTER_PASSWORD", onClose: A }), SUCCESS: e(ar, { animationProps: g, onClose: Y }), ALREADY_MIGRATED: e(It, { animationProps: g, isHidden: k !== "ALREADY_MIGRATED", onClose: Y }) }), [g, A, k, T, $, I, P, Z, O, Y]), { classes: le } = lr();
  return e(ee, { animationProps: { ...g, containerRef: p }, isContainer: true, children: E("div", { className: K(le.root, { "termius-dark-theme": true }), children: [q.INTRO, k !== "INTRO" && q.SETUP, !["INTRO", "SETUP", "ALREADY_MIGRATED"].includes(k) && ($.length || I.length) && q.UPDATE_DEVICES, !["INTRO", "SETUP", "UPDATE_DEVICES", "ALREADY_MIGRATED"].includes(k) && q.ENTER_PASSWORD, k === "SUCCESS" && q.SUCCESS, k === "ALREADY_MIGRATED" && q.ALREADY_MIGRATED] }) });
}, lr = z()({ ...Ce, root: { height: "100%", width: "100%", paddingBottom: "0", backgroundColor: "var(--dark-grey-1)", position: "absolute", top: "0", left: "0", right: "0", bottom: "0" } }), vr = yt(bt(cr));
export {
  vr as default
};
