/*
 * Recovered Termius UI module: useRequest-2d237c57.js
 * Readable reconstruction from the distributed bundle (no source map).
 * Original bindings, import specifiers and export names are preserved.
 * Local variable names and pre-bundle source boundaries cannot be recovered.
 */
import { r as h, mZ as H, I as p, C as b, zD as J, hC as y, M, aX as K, zE as $, L as V, G as X, K as Z, H as q, hD as z, J as Q, E as Y, ca as I, zF as C } from "./reconnectSaga-3b11f62e.js";
var U = function(e, n) {
  var r = n.manual, i = n.ready, u = i === void 0 ? true : i, a = n.defaultParams, o = a === void 0 ? [] : a, c = n.refreshDeps, s = c === void 0 ? [] : c, t = n.refreshDepsAction, v = h.useRef(false);
  return v.current = false, H(function() {
    !r && u && (v.current = true, e.run.apply(e, p([], b(o), false)));
  }, [u]), H(function() {
    v.current || r || (v.current = true, t ? t() : e.refresh());
  }, p([], b(s), false)), { onBefore: function() {
    if (!u) return { stopNow: true };
  } };
};
U.onInit = function(e) {
  var n = e.ready, r = n === void 0 ? true : n, i = e.manual;
  return { loading: !i && r };
};
const k = U;
function G(e, n) {
  var r = h.useRef({ deps: n, obj: void 0, initialized: false }).current;
  return (r.initialized === false || !J(r.deps, n)) && (r.deps = n, r.obj = e(), r.initialized = true), r.obj;
}
var O = /* @__PURE__ */ new Map(), ee = function(e, n, r) {
  var i = O.get(e);
  i != null && i.timer && clearTimeout(i.timer);
  var u = void 0;
  n > -1 && (u = setTimeout(function() {
    O.delete(e);
  }, n)), O.set(e, y(y({}, r), { timer: u }));
}, ne = function(e) {
  return O.get(e);
}, F = /* @__PURE__ */ new Map(), re = function(e) {
  return F.get(e);
}, te = function(e, n) {
  F.set(e, n), n.then(function(r) {
    return F.delete(e), r;
  }).catch(function() {
    F.delete(e);
  });
}, w = {}, ie = function(e, n) {
  w[e] && w[e].forEach(function(r) {
    return r(n);
  });
}, x = function(e, n) {
  return w[e] || (w[e] = []), w[e].push(n), function() {
    var i = w[e].indexOf(n);
    w[e].splice(i, 1);
  };
}, ue = function(e, n) {
  var r = n.cacheKey, i = n.cacheTime, u = i === void 0 ? 5 * 60 * 1e3 : i, a = n.staleTime, o = a === void 0 ? 0 : a, c = n.setCache, s = n.getCache, t = h.useRef(), v = h.useRef(), d = function(l, f) {
    c ? c(f) : ee(l, u, f), ie(l, f.data);
  }, g = function(l, f) {
    return f === void 0 && (f = []), s ? s(f) : ne(l);
  };
  return G(function() {
    if (r) {
      var l = g(r);
      l && Object.hasOwnProperty.call(l, "data") && (e.state.data = l.data, e.state.params = l.params, (o === -1 || (/* @__PURE__ */ new Date()).getTime() - l.time <= o) && (e.state.loading = false)), t.current = x(r, function(f) {
        e.setState({ data: f });
      });
    }
  }, []), M(function() {
    var l;
    (l = t.current) === null || l === void 0 || l.call(t);
  }), r ? { onBefore: function(l) {
    var f = g(r, l);
    return !f || !Object.hasOwnProperty.call(f, "data") ? {} : o === -1 || (/* @__PURE__ */ new Date()).getTime() - f.time <= o ? { loading: false, data: f == null ? void 0 : f.data, error: void 0, returnNow: true } : { data: f == null ? void 0 : f.data, error: void 0 };
  }, onRequest: function(l, f) {
    var m = re(r);
    return m && m !== v.current ? { servicePromise: m } : (m = l.apply(void 0, p([], b(f), false)), v.current = m, te(r, m), { servicePromise: m });
  }, onSuccess: function(l, f) {
    var m;
    r && ((m = t.current) === null || m === void 0 || m.call(t), d(r, { data: l, params: f, time: (/* @__PURE__ */ new Date()).getTime() }), t.current = x(r, function(D) {
      e.setState({ data: D });
    }));
  }, onMutate: function(l) {
    var f;
    r && ((f = t.current) === null || f === void 0 || f.call(t), d(r, { data: l, params: e.state.params, time: (/* @__PURE__ */ new Date()).getTime() }), t.current = x(r, function(m) {
      e.setState({ data: m });
    }));
  } } : {};
};
const ae = ue;
var oe = function(e, n) {
  var r = n.debounceWait, i = n.debounceLeading, u = n.debounceTrailing, a = n.debounceMaxWait, o = h.useRef(), c = h.useMemo(function() {
    var s = {};
    return i !== void 0 && (s.leading = i), u !== void 0 && (s.trailing = u), a !== void 0 && (s.maxWait = a), s;
  }, [i, u, a]);
  return h.useEffect(function() {
    if (r) {
      var s = e.runAsync.bind(e);
      return o.current = K(function(t) {
        t();
      }, r, c), e.runAsync = function() {
        for (var t = [], v = 0; v < arguments.length; v++) t[v] = arguments[v];
        return new Promise(function(d, g) {
          var l;
          (l = o.current) === null || l === void 0 || l.call(o, function() {
            s.apply(void 0, p([], b(t), false)).then(d).catch(g);
          });
        });
      }, function() {
        var t;
        (t = o.current) === null || t === void 0 || t.cancel(), e.runAsync = s;
      };
    }
  }, [r, c]), r ? { onCancel: function() {
    var s;
    (s = o.current) === null || s === void 0 || s.cancel();
  } } : {};
};
const se = oe;
var ce = function(e, n) {
  var r = n.loadingDelay, i = n.ready, u = h.useRef();
  if (!r) return {};
  var a = function() {
    u.current && clearTimeout(u.current);
  };
  return { onBefore: function() {
    return a(), i !== false && (u.current = setTimeout(function() {
      e.setState({ loading: true });
    }, r)), { loading: false };
  }, onFinally: function() {
    a();
  }, onCancel: function() {
    a();
  } };
};
const le = ce;
function B() {
  return $ ? document.visibilityState !== "hidden" : true;
}
var A = [];
function fe(e) {
  return A.push(e), function() {
    var r = A.indexOf(e);
    A.splice(r, 1);
  };
}
if ($) {
  var de = function() {
    if (B()) for (var e = 0; e < A.length; e++) {
      var n = A[e];
      n();
    }
  };
  window.addEventListener("visibilitychange", de, false);
}
var ve = function(e, n) {
  var r = n.pollingInterval, i = n.pollingWhenHidden, u = i === void 0 ? true : i, a = n.pollingErrorRetryCount, o = a === void 0 ? -1 : a, c = h.useRef(), s = h.useRef(), t = h.useRef(0), v = function() {
    var d;
    c.current && clearTimeout(c.current), (d = s.current) === null || d === void 0 || d.call(s);
  };
  return H(function() {
    r || v();
  }, [r]), r ? { onBefore: function() {
    v();
  }, onError: function() {
    t.current += 1;
  }, onSuccess: function() {
    t.current = 0;
  }, onFinally: function() {
    o === -1 || o !== -1 && t.current <= o ? c.current = setTimeout(function() {
      !u && !B() ? s.current = fe(function() {
        e.refresh();
      }) : e.refresh();
    }, r) : t.current = 0;
  }, onCancel: function() {
    v();
  } } : {};
};
const he = ve;
function ge(e, n) {
  var r = false;
  return function() {
    for (var i = [], u = 0; u < arguments.length; u++) i[u] = arguments[u];
    r || (r = true, e.apply(void 0, p([], b(i), false)), setTimeout(function() {
      r = false;
    }, n));
  };
}
function me() {
  return $ && typeof navigator.onLine < "u" ? navigator.onLine : true;
}
var S = [];
function pe(e) {
  return S.push(e), function() {
    var r = S.indexOf(e);
    r > -1 && S.splice(r, 1);
  };
}
if ($) {
  var N = function() {
    if (!(!B() || !me())) for (var e = 0; e < S.length; e++) {
      var n = S[e];
      n();
    }
  };
  window.addEventListener("visibilitychange", N, false), window.addEventListener("focus", N, false);
}
var be = function(e, n) {
  var r = n.refreshOnWindowFocus, i = n.focusTimespan, u = i === void 0 ? 5e3 : i, a = h.useRef(), o = function() {
    var c;
    (c = a.current) === null || c === void 0 || c.call(a);
  };
  return h.useEffect(function() {
    if (r) {
      var c = ge(e.refresh.bind(e), u);
      a.current = pe(function() {
        c();
      });
    }
    return function() {
      o();
    };
  }, [r, u]), M(function() {
    o();
  }), {};
};
const ye = be;
var Pe = function(e, n) {
  var r = n.retryInterval, i = n.retryCount, u = h.useRef(), a = h.useRef(0), o = h.useRef(false);
  return i ? { onBefore: function() {
    o.current || (a.current = 0), o.current = false, u.current && clearTimeout(u.current);
  }, onSuccess: function() {
    a.current = 0;
  }, onError: function() {
    if (a.current += 1, i === -1 || a.current <= i) {
      var c = r ?? Math.min(1e3 * Math.pow(2, a.current), 3e4);
      u.current = setTimeout(function() {
        o.current = true, e.refresh();
      }, c);
    } else a.current = 0;
  }, onCancel: function() {
    a.current = 0, u.current && clearTimeout(u.current);
  } } : {};
};
const Re = Pe;
var we = function(e, n) {
  var r = n.throttleWait, i = n.throttleLeading, u = n.throttleTrailing, a = h.useRef(), o = {};
  return i !== void 0 && (o.leading = i), u !== void 0 && (o.trailing = u), h.useEffect(function() {
    if (r) {
      var c = e.runAsync.bind(e);
      return a.current = V(function(s) {
        s();
      }, r, o), e.runAsync = function() {
        for (var s = [], t = 0; t < arguments.length; t++) s[t] = arguments[t];
        return new Promise(function(v, d) {
          var g;
          (g = a.current) === null || g === void 0 || g.call(a, function() {
            c.apply(void 0, p([], b(s), false)).then(v).catch(d);
          });
        });
      }, function() {
        var s;
        e.runAsync = c, (s = a.current) === null || s === void 0 || s.cancel();
      };
    }
  }, [r, i, u]), r ? { onCancel: function() {
    var c;
    (c = a.current) === null || c === void 0 || c.cancel();
  } } : {};
};
const Ce = we;
var Te = function() {
  var e = b(h.useState({}), 2), n = e[1];
  return h.useCallback(function() {
    return n({});
  }, []);
};
const Ae = Te;
var Se = (function() {
  function e(n, r, i, u) {
    u === void 0 && (u = {}), this.serviceRef = n, this.options = r, this.subscribe = i, this.initState = u, this.count = 0, this.state = { loading: false, params: void 0, data: void 0, error: void 0 }, this.state = y(y(y({}, this.state), { loading: !r.manual }), u);
  }
  return e.prototype.setState = function(n) {
    n === void 0 && (n = {}), this.state = y(y({}, this.state), n), this.subscribe();
  }, e.prototype.runPluginHandler = function(n) {
    for (var r = [], i = 1; i < arguments.length; i++) r[i - 1] = arguments[i];
    var u = this.pluginImpls.map(function(a) {
      var o;
      return (o = a[n]) === null || o === void 0 ? void 0 : o.call.apply(o, p([a], b(r), false));
    }).filter(Boolean);
    return Object.assign.apply(Object, p([{}], b(u), false));
  }, e.prototype.runAsync = function() {
    for (var n, r, i, u, a, o, c, s, t, v, d = [], g = 0; g < arguments.length; g++) d[g] = arguments[g];
    return X(this, void 0, void 0, function() {
      var l, f, m, D, L, j, W, E, P, R, _;
      return q(this, function(T) {
        switch (T.label) {
          case 0:
            if (this.count += 1, l = this.count, f = this.runPluginHandler("onBefore", d), m = f.stopNow, D = m === void 0 ? false : m, L = f.returnNow, j = L === void 0 ? false : L, W = z(f, ["stopNow", "returnNow"]), D) return [2, new Promise(function() {
            })];
            if (this.setState(y({ loading: true, params: d }, W)), j) return [2, Promise.resolve(W.data)];
            (r = (n = this.options).onBefore) === null || r === void 0 || r.call(n, d), T.label = 1;
          case 1:
            return T.trys.push([1, 3, , 4]), E = this.runPluginHandler("onRequest", this.serviceRef.current, d).servicePromise, E || (E = (_ = this.serviceRef).current.apply(_, p([], b(d), false))), [4, E];
          case 2:
            return P = T.sent(), l !== this.count ? [2, new Promise(function() {
            })] : (this.setState({ data: P, error: void 0, loading: false }), (u = (i = this.options).onSuccess) === null || u === void 0 || u.call(i, P, d), this.runPluginHandler("onSuccess", P, d), (o = (a = this.options).onFinally) === null || o === void 0 || o.call(a, d, P, void 0), l === this.count && this.runPluginHandler("onFinally", d, P, void 0), [2, P]);
          case 3:
            if (R = T.sent(), l !== this.count) return [2, new Promise(function() {
            })];
            throw this.setState({ error: R, loading: false }), (s = (c = this.options).onError) === null || s === void 0 || s.call(c, R, d), this.runPluginHandler("onError", R, d), (v = (t = this.options).onFinally) === null || v === void 0 || v.call(t, d, void 0, R), l === this.count && this.runPluginHandler("onFinally", d, void 0, R), R;
          case 4:
            return [2];
        }
      });
    });
  }, e.prototype.run = function() {
    for (var n = this, r = [], i = 0; i < arguments.length; i++) r[i] = arguments[i];
    this.runAsync.apply(this, p([], b(r), false)).catch(function(u) {
      n.options.onError || console.error(u);
    });
  }, e.prototype.cancel = function() {
    this.count += 1, this.setState({ loading: false }), this.runPluginHandler("onCancel");
  }, e.prototype.refresh = function() {
    this.run.apply(this, p([], b(this.state.params || []), false));
  }, e.prototype.refreshAsync = function() {
    return this.runAsync.apply(this, p([], b(this.state.params || []), false));
  }, e.prototype.mutate = function(n) {
    var r = Z(n) ? n(this.state.data) : n;
    this.runPluginHandler("onMutate", r), this.setState({ data: r });
  }, e;
})();
const De = Se;
function Ee(e, n, r) {
  n === void 0 && (n = {}), r === void 0 && (r = []);
  var i = n.manual, u = i === void 0 ? false : i, a = z(n, ["manual"]);
  Q && n.defaultParams && !Array.isArray(n.defaultParams) && console.warn("expected defaultParams is array, got ".concat(typeof n.defaultParams));
  var o = y({ manual: u }, a), c = Y(e), s = Ae(), t = G(function() {
    var v = r.map(function(d) {
      var g;
      return (g = d == null ? void 0 : d.onInit) === null || g === void 0 ? void 0 : g.call(d, o);
    }).filter(Boolean);
    return new De(c, o, s, Object.assign.apply(Object, p([{}], b(v), false)));
  }, []);
  return t.options = o, t.pluginImpls = r.map(function(v) {
    return v(t, o);
  }), I(function() {
    if (!u) {
      var v = t.state.params || n.defaultParams || [];
      t.run.apply(t, p([], b(v), false));
    }
  }), M(function() {
    t.cancel();
  }), { loading: t.state.loading, data: t.state.data, error: t.state.error, params: t.state.params || [], cancel: C(t.cancel.bind(t)), refresh: C(t.refresh.bind(t)), refreshAsync: C(t.refreshAsync.bind(t)), run: C(t.run.bind(t)), runAsync: C(t.runAsync.bind(t)), mutate: C(t.mutate.bind(t)) };
}
function Fe(e, n, r) {
  return Ee(e, n, p(p([], b(r || []), false), [se, le, he, ye, Ce, k, ae, Re], false));
}
export {
  Fe as u
};
