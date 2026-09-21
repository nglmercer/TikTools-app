import { expect, test } from 'bun:test';

import type { PluginPageDescriptor } from '../automation/behavior/types.ts';
import { adaptLegacyPage, legacyTtsContributions } from './legacy-v3-adapter.ts';

const text = { default: 'Title', i18key: '' };

function page(sections: PluginPageDescriptor['sections']): PluginPageDescriptor {
  return {
    id: 'main',
    pluginId: 'sonicboom.server',
    title: text,
    sections,
    source: { kind: 'plugin', pluginId: 'sonicboom.server' },
  };
}

test('adapts text, form, connection, and list sections to generic nodes', () => {
  const { page: adapted, tts } = adaptLegacyPage(
    page([
      { kind: 'text', title: text, text },
      { kind: 'form', title: text },
      { kind: 'connection' },
      {
        kind: 'list',
        title: text,
        optionsFrom: 'plugin-action-options:sonicboom.server.speak:voice',
      },
    ]),
  );
  expect(tts).toEqual([]);
  expect(adapted.body.type).toBe('stack');
  if (adapted.body.type !== 'stack') return;
  expect(adapted.body.children.map((node) => node.type)).toEqual([
    'text',
    'form',
    'connection',
    'list',
  ]);
  const list = adapted.body.children[3];
  expect(list?.type === 'list' && list.optionsFrom).toBe(
    'plugin-action-options:sonicboom.server.speak:voice',
  );
});

test('adapts tts sections to a contribution plus a tts-settings node', () => {
  const { page: adapted, tts } = adaptLegacyPage(
    page([
      {
        kind: 'tts',
        title: text,
        actionType: 'sonicboom.server.speak',
        voicesFrom: 'plugin-action-options:sonicboom.server.speak:voice',
        outputsFrom: 'plugin-action-options:sonicboom.server.set-output-device:device',
      },
    ]),
  );
  expect(tts).toEqual([
    {
      id: 'main',
      pluginId: 'sonicboom.server',
      actionType: 'sonicboom.server.speak',
      voicesFrom: 'plugin-action-options:sonicboom.server.speak:voice',
      outputsFrom: 'plugin-action-options:sonicboom.server.set-output-device:device',
    },
  ]);
  expect(adapted.body.type).toBe('stack');
  if (adapted.body.type !== 'stack') return;
  expect(adapted.body.children).toHaveLength(1);
  const node = adapted.body.children[0];
  expect(node?.type === 'tts-settings' && node.contribution).toBe('main');
});

test('derives contributions across pages without touching valid pages', () => {
  const pages = [
    page([
      {
        kind: 'tts',
        actionType: 'a.speak',
        voicesFrom: 'plugin-action-options:a.speak:voice',
      },
    ]),
    page([{ kind: 'text', text }]),
  ];
  const contributions = legacyTtsContributions(pages);
  expect(contributions).toHaveLength(1);
  expect(contributions[0]).toMatchObject({ id: 'main', actionType: 'a.speak' });
});

test('drops malformed option sources instead of emitting broken nodes', () => {
  const { page: adapted, tts } = adaptLegacyPage(
    page([
      { kind: 'list', optionsFrom: 'not a source!!!' },
      { kind: 'tts', actionType: '', voicesFrom: '' },
    ]),
  );
  expect(tts).toEqual([]);
  expect(adapted.body.type).toBe('stack');
  if (adapted.body.type !== 'stack') return;
  expect(adapted.body.children).toEqual([]);
});
