import { expect, test } from 'bun:test';

import {
  mergePluginPages,
  mergePluginTemplates,
  normalizeOptionsFrom,
  optionSourceId,
  parseOptionSourceId,
  parsePluginNavId,
  pluginNavId,
  splitTemplateId,
  toPluginPageDescriptor,
  toPluginTemplateDescriptor,
  isSecretField,
} from './declarative.ts';

test('option source ids round-trip', () => {
  expect(optionSourceId('sonicboom.server.speak', 'voice')).toBe(
    'plugin-action-options:sonicboom.server.speak:voice',
  );
  expect(parseOptionSourceId('plugin-action-options:sonicboom.server.speak:voice')).toEqual({
    actionType: 'sonicboom.server.speak',
    field: 'voice',
  });
  expect(parseOptionSourceId('voices')).toBeUndefined();
  expect(parseOptionSourceId('plugin-action-options:no-field')).toBeUndefined();
  expect(parseOptionSourceId('plugin-action-options:a:b:c')).toBeUndefined();
});

test('optionsFrom accepts strings and the object form', () => {
  expect(normalizeOptionsFrom('plugin-action-options:a.b:c')).toBe('plugin-action-options:a.b:c');
  expect(normalizeOptionsFrom('voices')).toBe('voices');
  expect(
    normalizeOptionsFrom({
      source: 'plugin-action-options',
      actionType: 'a.b',
      field: 'voice',
      optionsUrl: 'https://evil.example/voices',
    }),
  ).toBe('plugin-action-options:a.b:voice');
  expect(normalizeOptionsFrom({ source: 'other', actionType: 'a', field: 'b' })).toBeUndefined();
  expect(normalizeOptionsFrom('plugin-action-options:a:b:c')).toBeUndefined();
  expect(normalizeOptionsFrom('https://evil.example/x')).toBeUndefined();
  expect(normalizeOptionsFrom(undefined)).toBeUndefined();
});

test('secret detection reads schema and hints', () => {
  expect(isSecretField({ type: 'string', secret: true }, undefined)).toBe(true);
  expect(isSecretField({ type: 'string' }, { secret: true })).toBe(true);
  expect(isSecretField({ type: 'string' }, undefined)).toBe(false);
  expect(isSecretField(undefined, undefined)).toBe(false);
});

test('template and nav ids round-trip', () => {
  expect(splitTemplateId('sonicboom.server/chat-tts')).toEqual({
    pluginId: 'sonicboom.server',
    templateId: 'chat-tts',
  });
  expect(splitTemplateId('chat-tts')).toBeUndefined();
  expect(parsePluginNavId(pluginNavId('sonicboom.server', 'connection'))).toEqual({
    pluginId: 'sonicboom.server',
    pageId: 'connection',
  });
  expect(parsePluginNavId('feed')).toBeUndefined();
});

test('template descriptors reject malformed snapshots', () => {
  const valid = {
    id: 'sonicboom.server/chat-tts',
    pluginId: 'sonicboom.server',
    title: { default: 'Chat to TTS' },
    eventType: 'tiktok.chat',
    requiredNodeTypes: ['trigger.event', 'action.http'],
    workflow: { nodes: [{ type: 'trigger.event' }, { type: 'action.http', config: {} }] },
    source: { kind: 'plugin', pluginId: 'sonicboom.server' },
  };
  expect(toPluginTemplateDescriptor(valid)?.id).toBe('sonicboom.server/chat-tts');
  expect(toPluginTemplateDescriptor({ ...valid, title: { default: '' } })).toBeUndefined();
  expect(toPluginTemplateDescriptor({ ...valid, workflow: { nodes: [] } })).toBeUndefined();
  expect(toPluginTemplateDescriptor({ ...valid, requiredNodeTypes: [] })).toBeUndefined();
  expect(mergePluginTemplates(['chat-tts'], [valid, { ...valid }, 'nope'])).toHaveLength(1);
});

test('page descriptors enforce the fixed widget set', () => {
  const valid = {
    id: 'connection',
    pluginId: 'sonicboom.server',
    title: { default: 'Connection' },
    sections: [
      { kind: 'text', text: { default: 'Hello' } },
      { kind: 'form' },
      { kind: 'connection' },
      { kind: 'list', optionsFrom: 'plugin-action-options:a.b:c' },
    ],
    source: { kind: 'plugin', pluginId: 'sonicboom.server' },
  };
  expect(toPluginPageDescriptor(valid)?.sections).toHaveLength(4);
  for (const kind of ['html', 'script', 'component', 'iframe', 'webview']) {
    expect(
      toPluginPageDescriptor({ ...valid, sections: [{ kind }] }),
    ).toBeUndefined();
  }
  expect(
    toPluginPageDescriptor({ ...valid, sections: [{ kind: 'text' }] }),
  ).toBeUndefined();
  expect(
    toPluginPageDescriptor({ ...valid, sections: [{ kind: 'list' }] }),
  ).toBeUndefined();
  expect(mergePluginPages([valid, { ...valid }, 'nope'])).toHaveLength(1);
});
