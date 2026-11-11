import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const { groupSyncChanges, countLabel, collectionLabel } = require('../src/sync-change-groups.cjs');

const sample = (direction, action, kind, label, detail = '') => ({
  direction, action, kind, label, detail,
  text: direction + ': ' + action + ' ' + kind + ' ' + label + ' ' + detail,
});

test('Three deleted records become one group, with the host and two SSH configs kept visible', () => {
  const changes = [
    sample('upload', 'added', 'sshConfigs', 'запись 14'),
    sample('upload', 'modified', 'sshConfigs', '2001', '(другие параметры изменены; секреты скрыты)'),
    sample('upload', 'removed', 'hosts', 'Mybas Prod Secrets FTP'),
    sample('upload', 'removed', 'sshConfigs', '3322'),
    sample('upload', 'removed', 'sshConfigs', 'запись 15'),
  ];
  const groups = groupSyncChanges(changes);
  assert.equal(groups.length, 3);
  assert.deepEqual(groups.map(g => [g.action, g.total]), [
    ['added', 1], ['modified', 1], ['removed', 3],
  ]);
  const removed = groups[2];
  assert.equal(removed.countLabel, '3 записи');
  assert.equal(removed.directionLabel, 'На устройстве');
  assert.deepEqual(removed.categories.map(category => [category.label, category.total]), [
    ['Хост', 1], ['SSH-конфигурации', 2],
  ]);
  assert.deepEqual(removed.categories[0].entries.map(entry => entry.label), ['Mybas Prod Secrets FTP']);
  assert.deepEqual(removed.categories[1].entries.map(entry => entry.label), ['3322', 'запись 15']);
  assert.match(groups[1].categories[0].entries[0].detail, /секреты скрыты/);
});

test('Changes in opposite directions, different actions or conflicts never merge', () => {
  const grouped = groupSyncChanges([
    sample('upload', 'removed', 'hosts', 'A'),
    sample('download', 'removed', 'hosts', 'B'),
    sample('upload', 'added', 'hosts', 'C'),
    sample('conflict', 'conflict', 'hosts', 'D'),
    { direction: 'conflict', action: 'notice', text: 'Снимок удалён' },
  ]);
  assert.equal(grouped.length, 5);
  assert.deepEqual(grouped.map(g => [g.direction, g.action]), [
    ['upload','removed'], ['download','removed'], ['upload','added'],
    ['conflict','conflict'], ['conflict','notice'],
  ]);
  assert.equal(grouped[4].categories[0].entries[0].text, 'Снимок удалён');
});

test('Russian counters and category plurals remain readable', () => {
  assert.equal(countLabel(1), '1 запись');
  assert.equal(countLabel(2), '2 записи');
  assert.equal(countLabel(5), '5 записей');
  assert.equal(countLabel(11), '11 записей');
  assert.equal(countLabel(21), '21 запись');
  assert.equal(collectionLabel('sshConfigs', 1), 'SSH-конфигурация');
  assert.equal(collectionLabel('sshConfigs', 2), 'SSH-конфигурации');
  assert.equal(collectionLabel('sshConfigs', 5), 'SSH-конфигураций');
});

test('Sync dialog omits decorative copy but preserves meaningful controls and settings hints', () => {
  const ui = fs.readFileSync(new URL('../src/self-cloud-ui.cjs', import.meta.url), 'utf8');
  const css = fs.readFileSync(new URL('../src/self-cloud.css', import.meta.url), 'utf8');
  assert.ok(!ui.includes('Личная синхронизация · Ваш сервер'));
  assert.ok(!ui.includes('Используются настройки из конфига приложения'));
  assert.ok(!ui.includes('class="self-cloud-subtitle"'));
  assert.ok(!css.includes('.self-cloud-subtitle'));
  assert.ok(ui.includes('data-sync-preview hidden'), 'Change preview must remain');
  assert.ok(ui.includes('data-sync-warning hidden'), 'Overwrite warning must remain');
  assert.ok(ui.includes('data-confirm-sync hidden'), 'Manual sync confirmation must remain');
  assert.ok(ui.includes('Токен и пароль сохраняются в конфиге этого профиля'),
    'Useful configuration help must not be removed');
});

test('Modal groups changes with safe text, collapsible categories and no sync side effects', () => {
  const ui = fs.readFileSync(new URL('../src/self-cloud-ui.cjs', import.meta.url), 'utf8');
  const style = fs.readFileSync(new URL('../src/self-cloud.css', import.meta.url), 'utf8');
  assert.ok(ui.includes("require('./sync-change-groups.cjs')"));
  assert.ok(ui.includes("document.createElement('details')"));
  assert.ok(ui.includes("card.dataset.action = group.action"));
  assert.ok(ui.includes("category.label + (category.total > 1 ? ' ×' + category.total : '')"));
  assert.ok(ui.includes("names.textContent = category.entries.map(item => '«' + item.label + '»').join(', ')"));
  assert.ok(!ui.includes('entry.innerHTML = item.text'), 'Never render synchronized record labels as HTML');
  assert.ok(style.includes('.self-cloud-change-group'));
  assert.ok(style.includes('.self-cloud-change-category'));
});
