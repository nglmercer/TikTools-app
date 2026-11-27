import { expect, test } from 'vitest';

import type { PluginPageDescriptor } from '../../../../src/automation/behavior/types.ts';
import type { JsonValue } from '../../../../src/automation/types.ts';
import type { PluginUiDescriptor } from '../../../../src/plugin-ui/index.ts';
import {
  HOST_GIFTS_SOURCE,
  isConnectionOnlyPage,
  isHostOptionSource,
  mergePluginAutocomplete,
  mergePluginNavigationPages,
  mergePluginPages,
  mergePluginUis,
  normalizeOptionsFrom,
  optionFields,
  optionSourceId,
  parseOptionSourceId,
  parsePluginNavId,
  pluginNavId,
  splitTemplateId,
  toPluginAutocompleteContribution,
  toPluginPageDescriptor,
  isSecretField,
} from '../../../../src/automation/plugins/declarative.ts';
import { EVENT_KIND, PLUGIN_ID, PLUGIN_SOURCE } from '../../../shared/index.ts';

test('option source ids round-trip', () => {
  expect(optionSourceId(PLUGIN_ID.sonicboomServerSpeak, 'voice')).toBe(
    PLUGIN_SOURCE.sonicboomVoices,
  );
  expect(parseOptionSourceId(PLUGIN_SOURCE.sonicboomVoices)).toEqual({
    actionType: PLUGIN_ID.sonicboomServerSpeak,
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

test('host gift source is allowlisted in both marker forms', () => {
  expect(HOST_GIFTS_SOURCE).toBe('host.gifts');
  expect(isHostOptionSource('host.gifts')).toBe(true);
  expect(isHostOptionSource('host.state')).toBe(false);
  expect(isHostOptionSource('voices')).toBe(false);
  expect(normalizeOptionsFrom('host.gifts')).toBe('host.gifts');
  expect(normalizeOptionsFrom('  host.gifts  ')).toBe('host.gifts');
  expect(normalizeOptionsFrom({ source: 'host.gifts' })).toBe('host.gifts');
  // The host.* namespace fails closed: anything else is rejected even
  // though it matches the legacy token charset.
  expect(normalizeOptionsFrom('host.state')).toBeUndefined();
  expect(normalizeOptionsFrom('host.gifts.evil')).toBeUndefined();
  expect(normalizeOptionsFrom({ source: 'host.state' })).toBeUndefined();
  // Existing sources keep working unchanged.
  expect(normalizeOptionsFrom('voices')).toBe('voices');
  expect(optionFields({ fields: { giftName: { optionsFrom: 'host.gifts' } } })).toEqual([
    { key: 'giftName', source: 'host.gifts' },
  ]);
  expect(optionFields({ fields: { giftName: { optionsFrom: 'host.state' } } })).toEqual([]);
});

test('secret detection reads schema and hints', () => {
  expect(isSecretField({ type: 'string', secret: true }, undefined)).toBe(true);
  expect(isSecretField({ type: 'string' }, { secret: true })).toBe(true);
  expect(isSecretField({ type: 'string' }, undefined)).toBe(false);
  expect(isSecretField(undefined, undefined)).toBe(false);
});

test('template and nav ids round-trip', () => {
  expect(splitTemplateId('sonicboom.server/chat-tts')).toEqual({
    pluginId: PLUGIN_ID.sonicboomServer,
    templateId: 'chat-tts',
  });
  expect(splitTemplateId('chat-tts')).toBeUndefined();
  expect(parsePluginNavId(pluginNavId(PLUGIN_ID.sonicboomServer, 'connection'))).toEqual({
    pluginId: PLUGIN_ID.sonicboomServer,
    pageId: 'connection',
  });
  expect(parsePluginNavId('feed')).toBeUndefined();
});

test('page descriptors enforce the fixed widget set', () => {
  const valid = {
    id: 'connection',
    pluginId: PLUGIN_ID.sonicboomServer,
    title: { default: 'Connection' },
    sections: [
      { kind: 'text', text: { default: 'Hello' } },
      { kind: 'form' },
      { kind: 'connection' },
      { kind: 'list', optionsFrom: 'plugin-action-options:a.b:c' },
      { kind: 'tts', actionType: PLUGIN_ID.sonicboomServerSpeak, voicesFrom: PLUGIN_SOURCE.sonicboomVoices },
    ],
    source: { kind: 'plugin', pluginId: PLUGIN_ID.sonicboomServer },
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
    pluginId: PLUGIN_ID.sonicboomServer,
    apiVersion: 1,
    mode: 'webview',
    entry: 'ui/dist/index.html',
    pages: [{ id: 'tts', title: text }],
  };
  expect(mergePluginUis(undefined)).toEqual([]);
  const merged = mergePluginUis([valid, valid, { mode: 'nope' }, null]);
  expect(merged).toHaveLength(1);
  expect(merged[0]?.pluginId).toBe(PLUGIN_ID.sonicboomServer);
});

test('typed ui-only pages join navigation without duplicating legacy entries', () => {
  const title = { default: 'Text to Speech', i18key: '' };
  const legacy: PluginPageDescriptor = {
    id: 'tts',
    pluginId: PLUGIN_ID.sonicboomServer,
    title,
    icon: 'voice',
    sections: [{ kind: 'list', optionsFrom: 'plugin-action-options:a.b:c' }],
    source: { kind: 'plugin', pluginId: PLUGIN_ID.sonicboomServer },
  };
  const ui: PluginUiDescriptor = {
    pluginId: PLUGIN_ID.sonicboomServer,
    apiVersion: 1,
    mode: 'declarative',
    pages: [
      { id: 'tts', title },
      {
        id: 'main',
        title: { default: 'Main', i18key: '' },
        body: { type: 'stack', children: [{ type: 'text', text: { default: 'Hello', i18key: '' } }] },
      },
    ],
  };
  const merged = mergePluginNavigationPages([legacy], [ui]);
  expect(merged.map((page) => page.id)).toEqual(['tts', 'main']);
  expect(merged[0]).toMatchObject({ pluginId: PLUGIN_ID.sonicboomServer, connectionOnly: false });
  expect(merged[1]).toMatchObject({ pluginId: PLUGIN_ID.sonicboomServer, connectionOnly: false });
});

test('typed ui connection-only pages stay out of the sidebar projection', () => {
  const ui: PluginUiDescriptor = {
    pluginId: PLUGIN_ID.sonicboomServer,
    apiVersion: 1,
    mode: 'declarative',
    pages: [{
      id: 'connection',
      title: { default: 'Connection', i18key: '' },
      body: { type: 'stack', children: [
        { type: 'text', text: { default: 'Intro', i18key: '' } },
        { type: 'connection' },
      ] },
    }],
  };
  expect(mergePluginNavigationPages([], [ui])[0]?.connectionOnly).toBe(true);
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
    actionType: PLUGIN_ID.sonicboomServerSpeak,
    voicesFrom: PLUGIN_SOURCE.sonicboomVoices,
  };
  const page = (entry: JsonValue) => ({
    id: 'tts',
    pluginId: PLUGIN_ID.sonicboomServer,
    title: { default: 'TTS' },
    sections: [entry],
    source: { kind: 'plugin', pluginId: PLUGIN_ID.sonicboomServer },
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
    triggers: [EVENT_KIND.tiktokChat],
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
