'use strict';

const { execFileSync } = require('node:child_process');
const fs = require('node:fs');

const STABLE_TAG = /^v(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

function parseVersion(value) {
  const match = String(value).match(/^(?:v)?(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/);
  if (!match) throw new Error(`Invalid stable semantic version: ${value}`);
  return { major: Number(match[1]), minor: Number(match[2]), patch: Number(match[3]) };
}

function formatVersion(version) {
  return `${version.major}.${version.minor}.${version.patch}`;
}

function compareVersions(left, right) {
  return left.major - right.major || left.minor - right.minor || left.patch - right.patch;
}

function stableTags(tags) {
  return tags
    .filter((tag) => STABLE_TAG.test(tag))
    .map((tag) => ({ tag, version: parseVersion(tag) }))
    .sort((left, right) => compareVersions(right.version, left.version));
}

function selectRelease(tags, headTags, override) {
  const releases = stableTags(tags);
  const highest = releases[0] || null;
  const existingAtHead = stableTags(headTags)[0] || null;

  if (override) {
    const requested = parseVersion(override);
    const requestedTag = `v${formatVersion(requested)}`;
    const isHeadTag = headTags.includes(requestedTag);
    if (highest && compareVersions(requested, highest.version) < 0) {
      throw new Error(`Requested ${requestedTag} is lower than existing highest tag ${highest.tag}`);
    }
    if (highest && compareVersions(requested, highest.version) === 0 && !isHeadTag) {
      throw new Error(`${requestedTag} already exists on a different commit`);
    }
    return { version: formatVersion(requested), tag: requestedTag, reused: isHeadTag };
  }

  if (existingAtHead) {
    return { version: formatVersion(existingAtHead.version), tag: existingAtHead.tag, reused: true };
  }
  if (!highest) return { version: '1.0.0', tag: 'v1.0.0', reused: false };
  const next = { ...highest.version, patch: highest.version.patch + 1 };
  return { version: formatVersion(next), tag: `v${formatVersion(next)}`, reused: false };
}

function sanitizeSubject(subject) {
  return String(subject)
    .replace(/[\r\n]+/g, ' ')
    .replace(/([\\`*_{}\[\]()<>#+.!|~-])/g, '\\$1')
    .trim();
}

function makeChangelog(version, commits, previousTag) {
  const heading = `## Termian ${version}`;
  const range = previousTag ? `\n\nChanges since ${previousTag}:` : '\n\nInitial release changes:';
  const entries = commits.length
    ? commits.map(({ hash, subject }) => `- \`${hash.slice(0, 12)}\` ${sanitizeSubject(subject)}`).join('\n')
    : '- No new commits.';
  return `${heading}${range}\n\n${entries}`;
}

function git(args, options = {}) {
  return execFileSync('git', args, { encoding: 'utf8', ...options }).trim();
}

function lines(value) {
  return value ? value.split(/\r?\n/).filter(Boolean) : [];
}

function readCommits(previousTag) {
  const range = previousTag ? `${previousTag}..HEAD` : 'HEAD';
  const output = git(['log', range, '--format=%H%x00%s']);
  return lines(output).map((line) => {
    const separator = line.indexOf('\0');
    return { hash: line.slice(0, separator), subject: line.slice(separator + 1) };
  });
}

function appendGithubOutput(path, values) {
  const marker = `TERMIAN_CHANGELOG_${process.pid}_${Date.now()}`;
  const simple = ['version', 'tag', 'previousTag', 'reused', 'sha']
    .map((key) => `${key}=${values[key]}`)
    .join('\n');
  fs.appendFileSync(path, `${simple}\nchangelog<<${marker}\n${values.changelog}\n${marker}\n`);
}

function main(argv) {
  let override = '';
  let outputPath = process.env.GITHUB_OUTPUT || '';
  for (let index = 0; index < argv.length; index += 1) {
    if (argv[index] === '--version') override = argv[++index] || '';
    else if (argv[index] === '--github-output') outputPath = argv[++index] || '';
    else throw new Error(`Unknown argument: ${argv[index]}`);
  }

  const tags = lines(git(['tag', '--list']));
  const headTags = lines(git(['tag', '--points-at', 'HEAD']));
  const selected = selectRelease(tags, headTags, override);
  const selectedVersion = parseVersion(selected.version);
  const previous = stableTags(tags).find(({ version }) => compareVersions(version, selectedVersion) < 0) || null;
  const result = {
    ...selected,
    previousTag: previous ? previous.tag : '',
    sha: git(['rev-parse', 'HEAD']),
  };
  result.changelog = makeChangelog(result.version, readCommits(result.previousTag), result.previousTag);

  if (outputPath) appendGithubOutput(outputPath, result);
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
}

if (require.main === module) {
  try {
    main(process.argv.slice(2));
  } catch (error) {
    console.error(error.message);
    process.exitCode = 1;
  }
}

module.exports = {
  compareVersions,
  formatVersion,
  makeChangelog,
  parseVersion,
  sanitizeSubject,
  selectRelease,
  stableTags,
};
