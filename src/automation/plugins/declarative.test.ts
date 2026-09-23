import { expect, test } from 'bun:test';

import type { JsonValue } from '../types.ts';
import {
  isConnectionOnlyPage,
  mergePluginAutocomplete,
  mergePluginPages,
  mergePluginTemplates,
  mergePluginUis,
  normalizeOptionsFrom,
  optionSourceId,
  parseOptionSourceId,
  parsePluginNavId,
  pluginNavId,
  splitTemplateId,
  toPluginAutocompleteContribution,
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
      { kind: 'tts', actionType: 'sonicboom.server.speak', voicesFrom: 'plugin-action-options:sonicboom.server.speak:voice' },
    ],
    source: { kind: 'plugin', pluginId: 'sonicboom.server' },
  };
  expect(toPluginPageDescriptor(valid)?.sections).toHaveLength(5);
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
  expect(
    toPluginPageDescriptor({ ...valid, sections: [{ kind: 'tts', actionType: 'a.b' }] }),
  ).toBeUndefined();
  expect(
    toPluginPageDescriptor({ ...valid, sections: [{ kind: 'tts', voicesFrom: 'plugin-action-options:a.b:c' }] }),
  ).toBeUndefined();
  expect(mergePluginPages([valid, { ...valid }, 'nope'])).toHaveLength(1);
});

test('mergePluginUis drops invalid descriptors and dedupes by plugin', () => {
  const text = { default: 'Text to Speech', i18key: '' };
  const valid = {
    pluginId: 'sonicboom.server',
    apiVersion: 1,
    mode: 'webview',
    entry: 'ui/dist/index.html',
    pages: [{ id: 'tts', title: text }],
  };
  expect(mergePluginUis(undefined)).toEqual([]);
  const merged = mergePluginUis([valid, valid, { mode: 'nope' }, null]);
  expect(merged).toHaveLength(1);
  expect(merged[0]?.pluginId).toBe('sonicboom.server');
});

test('connection-only pages are owned by the Connections tab', () => {
  const sections = (...kinds: string[]) => kinds.map((kind) => ({ kind }));
  // Bare connection and text + connection (SonicBoom style) collapse into Connections.
  expect(isConnectionOnlyPage({ sections: sections('connection') })).toBe(true);
  expect(isConnectionOnlyPage({ sections: sections('text', 'connection') })).toBe(true);
  expect(isConnectionOnlyPage({ sections: sections('connection', 'connection') })).toBe(true);
  // Anything with its own editors stays a standalone tab.
  expect(isConnectionOnlyPage({ sections: sections('connection', 'form') })).toBe(false);
  expect(isConnectionOnlyPage({ sections: sections('text', 'connection', 'tts') })).toBe(false);
  expect(isConnectionOnlyPage({ sections: sections('connection', 'list') })).toBe(false);
  expect(isConnectionOnlyPage({ sections: sections('text') })).toBe(false);
  expect(isConnectionOnlyPage({ sections: sections('form') })).toBe(false);
  expect(isConnectionOnlyPage({ sections: [] })).toBe(false);
});

test('tts outputs source stays optional and never drops the panel', () => {
  const section = {
    kind: 'tts',
    actionType: 'sonicboom.server.speak',
    voicesFrom: 'plugin-action-options:sonicboom.server.speak:voice',
  };
  const page = (entry: JsonValue) => ({
    id: 'tts',
    pluginId: 'sonicboom.server',
    title: { default: 'TTS' },
    sections: [entry],
    source: { kind: 'plugin', pluginId: 'sonicboom.server' },
  });
  // Declared outputs enable the selector.
  expect(
    toPluginPageDescriptor(
      page({ ...section, outputsFrom: 'plugin-action-options:sonicboom.server.set-output-device:device' }),
    )?.sections[0]?.outputsFrom,
  ).toBe('plugin-action-options:sonicboom.server.set-output-device:device');
  // Omitted outputs keep the panel without a selector.
  expect(toPluginPageDescriptor(page(section))?.sections[0]?.outputsFrom).toBeUndefined();
  // Malformed markers hide the selector instead of dropping the panel.
  for (const outputsFrom of ['https://evil.example/x', '   ', 42]) {
    const parsed = toPluginPageDescriptor(page({ ...section, outputsFrom }));
    expect(parsed?.sections).toHaveLength(1);
    expect(parsed?.sections[0]?.outputsFrom).toBeUndefined();
  }
});

test('autocomplete contributions keep their claims and gain a source', () => {
  const parsed = toPluginAutocompleteContribution({
    id: 'textintel/stable-views',
    pluginId: 'textintel',
    prefixes: ['event.intel.comment.', '  event.intel.user.  '],
    triggers: ['tiktok.chat'],
    source: { kind: 'plugin', pluginId: 'textintel' },
  });
  expect(parsed?.id).toBe('textintel/stable-views');
  expect(parsed?.prefixes).toEqual(['event.intel.comment.', 'event.intel.user.']);
  expect(parsed?.source).toEqual({ kind: 'plugin', pluginId: 'textintel' });
  // A missing stamp is synthesized from the plugin id, like templates.
  const unstamped = toPluginAutocompleteContribution({
    id: 'demo/all',
    pluginId: 'demo',
    paths: ['event.intel.providers.demo.x'],
  });
  expect(unstamped?.source).toEqual({ kind: 'plugin', pluginId: 'demo' });
});

test('autocomplete fields need a path, a scalar kind, and a label', () => {
  const base = {
    id: 'demo/moderation',
    pluginId: 'demo',
    source: { kind: 'plugin', pluginId: 'demo' },
  };
  const field = {
    path: 'event.intel.providers.demo.comment.moderation.blocked',
    kind: 'boolean',
    label: { default: 'Blocked' },
    hint: { default: 'True when blocked.' },
  };
  expect(toPluginAutocompleteContribution({ ...base, fields: [field] })?.fields).toHaveLength(1);
  for (const broken of [
    { ...field, kind: 'object' },
    { ...field, label: undefined },
    { ...field, path: 'event.intel.providers.demo.x.' },
    { ...field, path: '  ' },
  ]) {
    expect(toPluginAutocompleteContribution({ ...base, fields: [broken] })).toBeUndefined();
  }
});

test('autocomplete merge drops invalid entries and duplicate ids', () => {
  expect(mergePluginAutocomplete(undefined)).toEqual([]);
  const valid = {
    id: 'demo/all',
    pluginId: 'demo',
    prefixes: ['event.intel.comment.'],
    source: { kind: 'plugin', pluginId: 'demo' },
  };
  const merged = mergePluginAutocomplete([
    valid,
    { ...valid, prefixes: ['event.intel.user.'] },
    { id: 'demo/empty', pluginId: 'demo' },
    { id: '', pluginId: 'demo', prefixes: ['event.intel.comment.'] },
    'not-an-object',
  ]);
  expect(merged.map((entry) => entry.id)).toEqual(['demo/all']);
  expect(merged[0]?.prefixes).toEqual(['event.intel.comment.']);
});
