import { expect, test } from 'bun:test';

import type { JsonValue } from '../shared/json.ts';
import { collectOptionSources, normalizeNode, normalizePage } from './normalize.ts';

const text = { default: 'Hello', i18key: '' };

test('normalizes a nested declarative page', () => {
  const page = normalizePage({
    id: 'tts',
    pluginId: 'sonicboom.server',
    title: text,
    icon: 'voice',
    body: {
      type: 'stack',
      children: [
        { type: 'text', text },
        {
          type: 'select',
          label: text,
          bind: 'settings.defaultVoice',
          optionsFrom: 'plugin-action-options:sonicboom.server.speak:voice',
        },
        { type: 'range', label: text, bind: 'settings.volume', min: 0, max: 1, step: 0.01 },
        { type: 'checkbox', label: text, bind: 'settings.enabled' },
        {
          type: 'button',
          label: text,
          action: { type: 'refresh-source', source: 'plugin-action-options:a:b' },
        },
        { type: 'list', optionsFrom: 'plugin-action-options:a:b' },
        { type: 'separator' },
        { type: 'connection' },
        { type: 'tts-settings', contribution: 'main' },
        { type: 'form', title: text },
        { type: 'status', text, tone: 'ok' },
        { type: 'card', children: [{ type: 'text', text }] },
      ],
    },
  });
  expect(page?.id).toBe('tts');
  expect(page?.body.type).toBe('stack');
  if (page?.body.type === 'stack') expect(page.body.children).toHaveLength(12);
});

test('rejects unknown node types, bad bindings, and over-deep trees', () => {
  expect(normalizeNode({ type: 'obs-scene' } as unknown as JsonValue)).toBeUndefined();
  expect(normalizeNode({ type: 'text' })).toBeUndefined();
  expect(normalizeNode({ type: 'select', bind: 'eval(x)' })).toBeUndefined();
  expect(normalizeNode({ type: 'range', bind: 'settings.v', min: 5, max: 1 })).toBeUndefined();
  expect(normalizeNode({ type: 'button', action: { type: 'rpc' } })).toBeUndefined();
  expect(normalizeNode({ type: 'list' })).toBeUndefined();
  expect(normalizeNode({ type: 'stack', children: [] })).toBeUndefined();
  expect(
    normalizePage({ id: 'x', pluginId: 'p', title: text, body: { type: 'text' } }),
  ).toBeUndefined();

  let deep: JsonValue = { type: 'text', text };
  for (let i = 0; i < 12; i += 1) deep = { type: 'stack', children: [deep] };
  expect(normalizeNode(deep)).toBeUndefined();
});

test('collects option sources from lists, selects, and refresh buttons', () => {
  const body = normalizeNode({
    type: 'stack',
    children: [
      { type: 'list', optionsFrom: 'source-a' },
      { type: 'select', bind: 'settings.v', optionsFrom: 'source-b' },
      { type: 'button', action: { type: 'refresh-source', source: 'source-a' } },
      { type: 'text', text },
    ],
  });
  expect(body && collectOptionSources(body)).toEqual(['source-a', 'source-b']);
});
