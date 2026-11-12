'use strict';

// Presentation-only grouping: never infer that separately stored records belong
// to the same host unless the snapshot provides a verified relationship.
// Group by direction + operation, then by entity type, preserving every record.
const ACTIONS = Object.freeze({
  added: 'Добавлено',
  removed: 'Удалено',
  modified: 'Изменено',
  conflict: 'Конфликт',
  uncertain: 'Требует проверки',
  notice: 'Предупреждение',
});
const DIRECTIONS = Object.freeze({
  upload: 'На устройстве',
  download: 'В облаке',
  conflict: 'Обе стороны',
  uncertain: 'Направление неизвестно',
});
const FORMS = Object.freeze({
  groups: ['Группа', 'Группы', 'Групп'],
  hosts: ['Хост', 'Хоста', 'Хостов'],
  identities: ['Идентификатор', 'Идентификатора', 'Идентификаторов'],
  sshKeys: ['SSH-ключ', 'SSH-ключа', 'SSH-ключей'],
  snippets: ['Сниппет', 'Сниппета', 'Сниппетов'],
  snippetPackages: ['Пакет сниппетов', 'Пакета сниппетов', 'Пакетов сниппетов'],
  portForwardingRules: ['Проброс портов', 'Правила проброса портов', 'Правил проброса портов'],
  knownHosts: ['Известный хост', 'Известных хоста', 'Известных хостов'],
  sshConfigs: ['SSH-конфигурация', 'SSH-конфигурации', 'SSH-конфигураций'],
  telnetConfigs: ['Telnet-конфигурация', 'Telnet-конфигурации', 'Telnet-конфигураций'],
  proxies: ['Прокси', 'Прокси', 'Прокси'],
  workspaceTemplates: ['Шаблон рабочей области', 'Шаблона рабочей области', 'Шаблонов рабочей области'],
  settings: ['Настройки', 'Настройки', 'Настройки'],
  other: ['Запись', 'Записи', 'Записей'],
});
function form(n) {
  const mod100 = Math.abs(n) % 100;
  const mod10 = mod100 % 10;
  return mod100 >= 11 && mod100 <= 14 ? 2 : mod10 === 1 ? 0 : mod10 >= 2 && mod10 <= 4 ? 1 : 2;
}
function countLabel(n) {
  return n + ' ' + ['запись', 'записи', 'записей'][form(n)];
}
function collectionLabel(kind, n) {
  return (FORMS[kind] || FORMS.other)[form(n)];
}
function groupSyncChanges(changes) {
  const grouped = new Map();
  for (const change of changes || []) {
    if (!change || typeof change !== 'object') continue;
    const direction = DIRECTIONS[change.direction] ? change.direction : 'uncertain';
    const action = ACTIONS[change.action] ? change.action : 'notice';
    const groupKey = direction + ':' + action;
    let group = grouped.get(groupKey);
    if (!group) {
      group = {
        direction,
        action,
        directionLabel: DIRECTIONS[direction],
        actionLabel: ACTIONS[action],
        total: 0,
        categories: [],
        categoryMap: new Map(),
      };
      grouped.set(groupKey, group);
    }
    group.total++;
    const kind = FORMS[change.kind] ? change.kind : 'other';
    let category = group.categoryMap.get(kind);
    if (!category) {
      category = { kind, entries: [] };
      group.categoryMap.set(kind, category);
      group.categories.push(category);
    }
    category.entries.push({
      label: String(change.label || 'ранее сохранённая запись'),
      detail: String(change.detail || '').replace(/^\s*\(|\)\s*$/g, ''),
      // For notices or older changes without structured metadata.
      text: String(change.text || ''),
      hasLabel: typeof change.label === 'string' && change.label.length > 0,
    });
  }
  return [...grouped.values()].map(group => ({
    direction: group.direction,
    action: group.action,
    actionLabel: group.actionLabel,
    directionLabel: group.directionLabel,
    total: group.total,
    countLabel: countLabel(group.total),
    categories: group.categories.map(category => ({
      kind: category.kind,
      label: collectionLabel(category.kind, category.entries.length),
      total: category.entries.length,
      entries: category.entries,
    })),
  }));
}
module.exports = { groupSyncChanges, countLabel, collectionLabel };
