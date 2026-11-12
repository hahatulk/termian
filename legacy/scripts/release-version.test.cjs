'use strict';

const assert = require('node:assert/strict');
const { execFileSync } = require('node:child_process');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const test = require('node:test');
const {
  makeChangelog,
  parseVersion,
  sanitizeSubject,
  selectRelease,
  stableTags,
} = require('./release-version.cjs');

const script = path.join(__dirname, 'release-version.cjs');

function git(cwd, args) {
  return execFileSync('git', args, { cwd, encoding: 'utf8' }).trim();
}

test('accepts only stable SemVer versions and sorts numerically', () => {
  assert.deepEqual(parseVersion('v1.20.3'), { major: 1, minor: 20, patch: 3 });
  assert.throws(() => parseVersion('1.0.0-beta.1'), /Invalid/);
  assert.deepEqual(stableTags(['v1.9.0', 'junk', 'v1.10.0']).map((entry) => entry.tag), ['v1.10.0', 'v1.9.0']);
});

test('starts at 1.0.0, increments patches, and is idempotent at tagged HEAD', () => {
  assert.deepEqual(selectRelease([], [], ''), { version: '1.0.0', tag: 'v1.0.0', reused: false });
  assert.deepEqual(selectRelease(['v1.0.0'], [], ''), { version: '1.0.1', tag: 'v1.0.1', reused: false });
  assert.deepEqual(selectRelease(['v1.0.0'], ['v1.0.0'], ''), { version: '1.0.0', tag: 'v1.0.0', reused: true });
});

test('manual versions cannot move or reuse a tag on another commit', () => {
  assert.throws(() => selectRelease(['v2.0.0'], [], '1.9.9'), /lower/);
  assert.throws(() => selectRelease(['v2.0.0'], [], '2.0.0'), /different commit/);
  assert.equal(selectRelease(['v2.0.0'], [], '2.1.0').tag, 'v2.1.0');
});

test('changelog escapes commit subjects as Markdown', () => {
  assert.equal(sanitizeSubject('fix [link](x)\nnext'), 'fix \\[link\\]\\(x\\) next');
  assert.match(makeChangelog('1.0.0', [{ hash: 'abcdef1234567890', subject: 'add *thing*' }], ''), /`abcdef123456` add \\\*thing\\\*/);
});

test('CLI reads tags and commit messages safely from a temporary repository', () => {
  const cwd = fs.mkdtempSync(path.join(os.tmpdir(), 'termian-version-'));
  git(cwd, ['init', '-q']);
  git(cwd, ['config', 'user.email', 'ci@example.invalid']);
  git(cwd, ['config', 'user.name', 'CI']);
  fs.writeFileSync(path.join(cwd, 'file.txt'), 'one');
  git(cwd, ['add', 'file.txt']);
  git(cwd, ['commit', '-qm', 'initial [safe]']);
  git(cwd, ['tag', 'v1.0.0']);
  fs.appendFileSync(path.join(cwd, 'file.txt'), '\ntwo');
  git(cwd, ['commit', '-qam', 'second `safe`']);

  const result = JSON.parse(execFileSync(process.execPath, [script], { cwd, encoding: 'utf8' }));
  assert.equal(result.version, '1.0.1');
  assert.equal(result.previousTag, 'v1.0.0');
  assert.match(result.changelog, /second \\`safe\\`/);
});
