import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import YAML from 'yaml';

const root = path.resolve(import.meta.dirname, '../..');
const read = p => fs.readFileSync(path.join(root, p), 'utf8');
const baseConfig = JSON.parse(read('legacy/src/tauri.conf.json'));
const workflow = YAML.parse(read('.github/workflows/release.yml'));

test('macOS ARM64 release verifies signatures and supports Developer ID notarization', () => {
  const macStep = workflow.jobs.build.steps.find(step => step.name === 'Build and verify signed Apple Silicon DMG');
  assert.ok(macStep);
  assert.equal(baseConfig.bundle.macOS.signingIdentity, '-');
  assert.equal(macStep.env.APPLE_SIGNING_IDENTITY,
    '$' + "{{ secrets.APPLE_SIGNING_IDENTITY || '-' }}");
  for (const secret of ['APPLE_CERTIFICATE', 'APPLE_CERTIFICATE_PASSWORD', 'APPLE_ID', 'APPLE_PASSWORD', 'APPLE_TEAM_ID']) {
    assert.ok(macStep.env[secret]?.includes('secrets.' + secret));
  }
  assert.match(macStep.run, /codesign --verify --deep --verbose=2/);
  assert.match(macStep.run, /spctl --assess --verbose --type execute/);
  assert.match(macStep.run, /Partial Apple signing credentials/);
  assert.match(macStep.run, /node scripts\/publish-mac\.mjs/);
  assert.match(macStep.run, /Missing \$name for Apple Developer signing\/notarization/);
  assert.doesNotMatch(macStep.run, /xattr -dr com\.apple\.quarantine/);
});
