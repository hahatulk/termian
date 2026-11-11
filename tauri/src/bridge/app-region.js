// Electron drag regions of the recovered titlebar (`-webkit-app-region`).
// WebView2 (Chromium) keeps the property, so getComputedStyle reports it.
// WebKit (macOS WKWebView) drops it while parsing, so there the rules are
// recorded as emotion inserts them (`.tss-…-root{-webkit-app-region:drag;}`)
// and elements are matched against their selectors.
const RULE = /([^{}]+)\{[^{}]*?-webkit-app-region\s*:\s*(no-drag|drag)/gi;

export function parseAppRegionRules(text) {
  const rules = [];
  if (typeof text !== 'string' || !text.includes('-webkit-app-region')) return rules;
  for (const [, selector, region] of text.matchAll(RULE)) {
    const trimmed = selector.trim();
    if (trimmed && !trimmed.startsWith('@')) rules.push({selector: trimmed, region: region.toLowerCase()});
  }
  return rules;
}

export function installAppRegionTracker(win = window) {
  const inserted = [];
  const proto = win.CSSStyleSheet?.prototype;
  const insertRule = proto?.insertRule;
  if (insertRule) {
    proto.insertRule = function(rule, ...rest) {
      inserted.push(...parseAppRegionRules(rule));
      return insertRule.call(this, rule, ...rest);
    };
  }
  // Non-speedy emotion and static styles keep the rule text in <style> tags.
  const styleRules = new WeakMap();
  const textRules = () => {
    const rules = [];
    for (const style of win.document.querySelectorAll('style')) {
      const text = style.textContent ?? '';
      let cached = styleRules.get(style);
      if (cached?.text !== text) styleRules.set(style, cached = {text, rules: parseAppRegionRules(text)});
      rules.push(...cached.rules);
    }
    return rules;
  };
  const matches = (element, selector) => {
    try { return element.matches(selector); } catch { return false; }
  };

  // The element's own region: 'drag', 'no-drag' or '' when it sets none.
  function regionOf(element) {
    const computed = win.getComputedStyle(element).getPropertyValue('-webkit-app-region').trim();
    if (computed === 'drag' || computed === 'no-drag') return computed;
    const rules = [...textRules(), ...inserted];
    for (let index = rules.length - 1; index >= 0; index--) {
      if (matches(element, rules[index].selector)) return rules[index].region;
    }
    return '';
  }

  return {
    regionOf,
    isDragRegion(target) {
      for (let element = target?.nodeType === 1 ? target : null; element; element = element.parentElement) {
        const region = regionOf(element);
        if (region === 'no-drag') return false;
        if (region === 'drag') return true;
      }
      return false;
    },
  };
}
