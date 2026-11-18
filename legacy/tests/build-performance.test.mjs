import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import YAML from 'yaml';

const root = path.resolve(import.meta.dirname, '../..');
const read = pathname => fs.readFileSync(path.join(root, pathname), 'utf8');
const workflow = YAML.parse(read('.github/workflows/release.yml'));
const build = workflow.jobs.build;
const steps = build.steps;

test('Rust CI cache follows the real Cargo target directory for both desktop platforms', () => {
  const desktopCache = steps.find(s => s.uses?.toLowerCase().includes('rust-cache'));
  assert.ok(desktopCache);
  assert.equal(desktopCache.with.workspaces, 'legacy/src -> ../dist/.cargo');
  assert.equal(desktopCache.with['cache-targets'], true);
  const cli = read('legacy/scripts/tauri-cli.mjs');
  assert.ok(cli.includes("CARGO_TARGET_DIR: path.join(root, 'dist', '.cargo')"));
  assert.ok(fs.existsSync(path.join(root, 'legacy/src/Cargo.lock')));
});

test('Rust tests reuse release dependencies instead of compiling the debug profile first', () => {
  const rustTest = steps.find(s => s.name?.startsWith('Test Rust backend'));
  assert.ok(rustTest);
  assert.match(rustTest.run, /cargo test --release --lib --manifest-path src\/Cargo\.toml --locked/);
  assert.ok(rustTest.env.CARGO_TARGET_DIR.endsWith('/legacy/dist/.cargo'));
  assert.equal(steps.filter(s => /cargo test\b/.test(s.run || '')).length, 1);
  assert.ok(!steps.some(s => /dist\/\.cargo\/debug|cargo clean\b/.test(s.run || '')));
  assert.ok(steps.some(s => s.name?.includes('Build Windows portable EXE')));
});

test('macOS Rust tests and DMG build share the same explicit Apple Silicon target and settings', () => {
  const mac = build.strategy.matrix.include.find(entry => entry.platform === 'mac');
  assert.equal(mac.os, 'macos-15');
  assert.equal(mac.arch, 'arm64');
  const rustTest = steps.find(s => s.name?.startsWith('Test Rust backend'));
  assert.match(rustTest.run, /cargo test --release --lib --target aarch64-apple-darwin/);
  const dmg = steps.find(s => s.name?.includes('Apple Silicon DMG'));
  assert.match(dmg.run, /build --no-bundle --target aarch64-apple-darwin/,
    'Compile the Rust binary once before packaging');
  assert.match(dmg.run, /bundle --bundles app,dmg --target aarch64-apple-darwin/,
    'Preserve the .app bundle for codesign and spctl verification');
  assert.match(dmg.run, /app="dist\/\.cargo\/aarch64-apple-darwin\/release\/bundle\/macos\/termian\.app"/);
  assert.match(dmg.run, /\[\[ -d "\$app" \]\]/);
  assert.equal(build.env.MACOSX_DEPLOYMENT_TARGET,
    "${{ matrix.platform == 'mac' && '11.0' || '' }}");
  const desktopCache = steps.find(s => s.uses?.includes('rust-cache'));
  assert.equal(desktopCache.with['cache-on-failure'], true);
});

test('macOS DMG packaging retries only transient hdiutil Resource busy failures without recompilation', () => {
  const dmg = steps.find(s => s.name?.includes('Apple Silicon DMG'));
  assert.ok(dmg);
  assert.match(dmg.run, /for attempt in 1 2 3; do/);
  assert.match(dmg.run, /grep -Eqi 'hdiutil: .*failed - Resource busy'/);
  assert.match(dmg.run, /sleep "\$\(\(attempt \* 15\)\)"/);
  assert.match(dmg.run, /if \(\( attempt == 3 \)\); then/);
  assert.ok(dmg.run.indexOf('build --no-bundle') < dmg.run.indexOf('for attempt in 1 2 3'));
  assert.ok(!dmg.run.slice(dmg.run.indexOf('for attempt in 1 2 3')).includes('tauri-cli.mjs build '));
  assert.match(dmg.run, /node scripts\/tauri-cli\.mjs bundle --bundles app,dmg/);
});

test('macOS ad-hoc signing does not pass empty certificate secrets to Tauri', () => {
  const dmg = steps.find(s => s.name?.includes('Apple Silicon DMG'));
  assert.ok(dmg);
  assert.equal(dmg.env.APPLE_SIGNING_IDENTITY, "${{ secrets.APPLE_SIGNING_IDENTITY || '-' }}");
  // GitHub Actions puts missing secrets into the environment as empty strings.
  // If APPLE_CERTIFICATE remains set, Tauri still tries importing a PKCS#12.
  const unset = 'unset APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID';
  assert.match(dmg.run, /else\n[\s\S]*?unset APPLE_CERTIFICATE APPLE_CERTIFICATE_PASSWORD APPLE_ID APPLE_PASSWORD APPLE_TEAM_ID\n[\s\S]*?fi\n/);
  assert.ok(dmg.run.indexOf(unset) < dmg.run.indexOf('node scripts/tauri-cli.mjs build '));
  assert.match(dmg.run, /codesign --verify --deep/);
  assert.match(dmg.run, /spctl --assess --verbose --type execute/);
});

test('Publishing EXE and DMG preserves Cargo cache unless explicitly opted out', () => {
  for (const file of ['legacy/scripts/publish-windows.mjs', 'legacy/scripts/publish-mac.mjs']) {
    const source = read(file);
    assert.ok(source.includes("process.env.TERMIAN_CLEAN_BUILD_CACHE === '1'"),
      'Only opt-in cleanup may delete Rust cache: ' + file);
    assert.ok(source.includes("fs.rmSync("), 'Generated web files still need cleanup');
  }
  const scripts = JSON.parse(read('legacy/package.json')).scripts;
  assert.match(scripts['build:windows'], /publish-windows\.mjs/);
  assert.equal(scripts['clean:rust-cache'], 'node scripts/clean-rust-cache.mjs');
  const cleaner = read('legacy/scripts/clean-rust-cache.mjs');
  assert.ok(cleaner.includes("'dist', '.cargo'"));
  assert.ok(!cleaner.includes('termian-data'));
});

test('Cold release profile prefers faster compiler optimization without enabling debug mode', () => {
  const cargo = read('legacy/src/Cargo.toml');
  assert.match(cargo, /\[profile\.release\]\s+opt-level = 2\s+codegen-units = 32\s+lto = false/);
});

test('Release publishes only Windows and macOS artifacts', () => {
  assert.ok(!workflow.jobs.android);
  assert.deepEqual(workflow.jobs.publish.needs, ['prepare', 'build']);
  const config = JSON.parse(read('legacy/src/tauri.conf.json'));
  assert.ok(!config.bundle.android);
  for (const removed of ['legacy/src/mobile.js', 'legacy/src/tauri.android.conf.json',
    'legacy/src/ui/mobile.html', 'legacy/src/ui/mobile.css',
    'legacy/scripts/configure-android.mjs']) {
    assert.ok(!fs.existsSync(path.join(root, removed)), 'Android-only file remains: ' + removed);
  }
  const release = workflow.jobs.publish.steps.find(s => s.name?.includes('both desktop'));
  assert.ok(release);
  assert.ok(release.run.includes('-win-x64.exe'));
  assert.ok(release.run.includes('-win-x64-setup.exe'));
  assert.ok(release.run.includes('-mac-arm64.dmg'));
  assert.ok(!release.run.includes('.apk'));
});

test('A transient artifact service timeout retries the upload instead of failing the build', () => {
  const uploads = steps.filter(s => s.uses === 'actions/upload-artifact@v4');
  assert.equal(uploads.length, 3);
  assert.deepEqual(uploads.map(s => s['continue-on-error']), [true, true, false],
    'only the last attempt may fail the job');
  assert.equal(uploads[1].if, "steps.upload1.outcome == 'failure'");
  assert.ok(uploads.slice(1).every(s => s.with.overwrite === true), 'retries replace a partial artifact');
  assert.ok(new Set(uploads.map(s => s.with.name)).size === 1);
});
