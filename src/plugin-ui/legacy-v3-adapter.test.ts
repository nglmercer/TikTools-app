import { expect, test } from 'bun:test';

import type { PluginPageDescriptor } from '../automation/behavior/types.ts';
import { adaptLegacyPage } from './legacy-v3-adapter.ts';

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
  const adapted = adaptLegacyPage(
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

test('adapts legacy tts sections to a neutral status note', () => {
  const adapted = adaptLegacyPage(
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
  expect(adapted.body.type).toBe('stack');
  if (adapted.body.type !== 'stack') return;
  expect(adapted.body.children).toHaveLength(1);
  const node = adapted.body.children[0];
  expect(node?.type).toBe('status');
  if (node?.type !== 'status') return;
  expect(node.tone).toBe('info');
  expect(node.text.default).toContain('moved to the plugin view');
});

test('drops malformed option sources instead of emitting broken nodes', () => {
  const adapted = adaptLegacyPage(
    page([
      { kind: 'list', optionsFrom: 'not a source!!!' },
      { kind: 'tts', actionType: '', voicesFrom: '' },
    ]),
  );
  expect(adapted.body.type).toBe('stack');
  if (adapted.body.type !== 'stack') return;
  // The broken list is dropped; the legacy marker still degrades visibly.
  expect(adapted.body.children.map((node) => node.type)).toEqual(['status']);
});
