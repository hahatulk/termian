import { test } from 'node:test';
import assert from 'node:assert/strict';
import { installAppRegionTracker, parseAppRegionRules } from '../src/bridge/app-region.js';

test('Emotion app-region rules are parsed with their selectors', () => {
  assert.deepEqual(parseAppRegionRules('.tss-1-TitleBar-root{-webkit-app-region:drag;}'),
    [{selector: '.tss-1-TitleBar-root', region: 'drag'}]);
  assert.deepEqual(parseAppRegionRules('.a,.b:hover{color:red;-webkit-app-region: no-drag;}.c{color:blue}'),
    [{selector: '.a,.b:hover', region: 'no-drag'}]);
  assert.deepEqual(parseAppRegionRules('.c{color:blue}'), []);
});

// WebKit drops -webkit-app-region, so computed style is always empty there.
function fakeWebKitWindow() {
  class CSSStyleSheet { insertRule() { return 0; } }
  const element = (classes, parent = null) => ({
    nodeType: 1, parentElement: parent,
    matches: selector => selector.split(',').some(part => classes.includes(part.trim().slice(1))),
  });
  return {
    CSSStyleSheet, element,
    document: {querySelectorAll: () => [{textContent: '.static-drag{-webkit-app-region:drag}'}]},
    getComputedStyle: () => ({getPropertyValue: () => ''}),
  };
}

test('Drag regions are found on macOS WebKit where computed style lacks them', () => {
  const win = fakeWebKitWindow();
  const regions = installAppRegionTracker(win);
  const sheet = new win.CSSStyleSheet();
  sheet.insertRule('.titlebar{-webkit-app-region:drag;}', 0);
  sheet.insertRule('.button{-webkit-app-region:no-drag;}', 1);
  const titlebar = win.element(['titlebar']);
  const tab = win.element(['tab'], titlebar);
  const button = win.element(['button'], tab);
  assert.equal(regions.isDragRegion(titlebar), true);
  assert.equal(regions.isDragRegion(tab), true);
  assert.equal(regions.isDragRegion(button), false);
  assert.equal(regions.isDragRegion(win.element(['content'])), false);
  assert.equal(regions.isDragRegion(win.element(['static-drag'])), true);
});

test('Chromium computed app-region still decides on Windows', () => {
  const win = fakeWebKitWindow();
  win.getComputedStyle = () => ({getPropertyValue: () => 'drag'});
  assert.equal(installAppRegionTracker(win).isDragRegion(win.element(['anything'])), true);
});
