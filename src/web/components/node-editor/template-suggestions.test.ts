import { afterEach, describe, expect, test } from 'bun:test';

import {
  resetAutocompleteRegistry,
  syncAutocompletePluginStates,
} from '../../../automation/autocomplete-registry.ts';
import { sampleEventForType } from '../../../automation/event-registry.ts';
import type { JsonObject } from '../../../automation/types.ts';
import { applyPresetInsert } from '../autocomplete/autocomplete-controller.ts';
import {
  applyFetchUrlTemplate,
  getFetchUrlTemplates,
  getTemplateSuggestions,
  isLocalFetchUrl,
  registerFetchUrlTemplate,
} from './template-suggestions.ts';

test('http-url scope offers identity variables for query templating', () => {
  const suggestions = getTemplateSuggestions('tiktok.chat', 'en', undefined, 'http-url');
  expect(suggestions.some((entry) => entry.value === 'event.user.uniqueId')).toBe(true);
  expect(suggestions.some((entry) => entry.value === 'event.data.comment')).toBe(true);
});

test('template autocomplete discovers processor TTS views', () => {
  const suggestions = getTemplateSuggestions('tiktok.chat', 'en');
  expect(suggestions.some((entry) => entry.value === 'event.intel.comment.tts.text')).toBe(true);
  expect(suggestions.some((entry) => entry.value === 'event.intel.user.nickname.tts.text')).toBe(true);
  expect(suggestions.some((entry) => entry.value === 'event.intel.comment.language.top')).toBe(true);
});

test('builtin URL presets exist', () => {
  const presets = getFetchUrlTemplates();
  expect(presets.some((entry) => entry.url === 'http://localhost:3000/')).toBe(true);
  expect(presets.some((entry) => entry.url === 'https://')).toBe(true);
});

test('registerFetchUrlTemplate adds and replaces custom presets', () => {
  registerFetchUrlTemplate({ id: 'test-custom', label: 'custom', url: 'https://example.com/hook' });
  expect(getFetchUrlTemplates().some((entry) => entry.id === 'test-custom')).toBe(true);
  registerFetchUrlTemplate({ id: 'test-custom', label: 'custom-2', url: 'https://example.com/other' });
  const found = getFetchUrlTemplates().filter((entry) => entry.id === 'test-custom');
  expect(found).toHaveLength(1);
  expect(found[0]?.url).toBe('https://example.com/other');
});

test('registerFetchUrlTemplate rejects bad input', () => {
  expect(() => registerFetchUrlTemplate({ id: '', label: 'x', url: 'https://example.com/' })).toThrow();
  expect(() => registerFetchUrlTemplate({ id: 'bad', label: 'x', url: 'ftp://example.com/' })).toThrow();
});

test('isLocalFetchUrl matches loopback and LAN, not public hosts', () => {
  for (const url of [
    'http://localhost:3000/',
    'http://127.0.0.1:8000/hook',
    'http://192.168.1.100:3000/',
    'http://10.0.0.5/',
    'http://[::1]:3000/',
  ]) expect(isLocalFetchUrl(url)).toBe(true);
  for (const url of ['https://hooks.example.com/live', 'https://discord.com/api/webhooks/x', '', 'not a url']) {
    expect(isLocalFetchUrl(url)).toBe(false);
  }
});

test('applyFetchUrlTemplate keeps path and query, swaps origin', () => {
  expect(applyFetchUrlTemplate('https://old.example.com/a?x=1', 'http://localhost:3000/')).toBe(
    'http://localhost:3000/a?x=1',
  );
  expect(applyFetchUrlTemplate('', 'http://localhost:3000/')).toBe('http://localhost:3000/');
  expect(applyFetchUrlTemplate('https://', 'http://localhost:3000/')).toBe('http://localhost:3000/');
  expect(applyFetchUrlTemplate('https://old.example.com', 'http://localhost:3000/')).toBe('http://localhost:3000/');
});

test('applyFetchUrlTemplate matches the shared preset insert exactly', () => {
  const cases: Array<[string, string]> = [
    ['', 'http://localhost:3000/'],
    ['https://', 'http://localhost:3000/'],
    ['https://old.example.com', 'http://localhost:3000/'],
    ['https://old.example.com/a?x=1#f', 'http://localhost:3000/'],
    ['http://127.0.0.1:8000/hook?q=1', 'https://hooks.example.com/live'],
    ['not a url', 'http://localhost:3000/'],
  ];
  for (const [current, preset] of cases) {
    expect(applyFetchUrlTemplate(current, preset)).toBe(applyPresetInsert(current, preset).value);
  }
});

test('hover cards show concise protobuf provenance', () => {
  const suggestions = getTemplateSuggestions('tiktok.chat', 'en');
  const comment = suggestions.find((entry) => entry.value === 'event.data.comment');
  expect(comment?.documentation).toContain(
    'protobuf WebcastChatMessage.content: string → event.data.comment: string · native',
  );
  const gift = getTemplateSuggestions('tiktok.gift', 'en');
  const giftId = gift.find((entry) => entry.value === 'event.data.giftId');
  expect(giftId?.documentation).toContain(
    'protobuf WebcastGiftMessage.gift_id: int64 → event.data.giftId: string · u64-to-string',
  );
});

test('hover cards trace nested fields to their protobuf source', () => {
  const gift = getTemplateSuggestions('tiktok.gift', 'en');
  const diamonds = gift.find((entry) => entry.value === 'event.data.diamondCount');
  expect(diamonds?.documentation).toContain(
    'protobuf WebcastGiftMessage.gift.diamond_count: int32 → event.data.diamondCount: number · normalized-unsigned',
  );
});

test('TikTools-only fields show no protobuf provenance', () => {
  const suggestions = getTemplateSuggestions('tiktok.gift', 'en');
  const method = suggestions.find((entry) => entry.value === 'event.data.method');
  expect(method?.documentation ?? '').not.toContain('protobuf ');
  // The fallback names the normalized DTO, never the vendor message.
  expect(method?.documentation ?? '').toContain('native GiftAutomationData.method');
});

test('observed-path fallback still surfaces live-only paths without duplicating registry ones', () => {
  const lastEvent = sampleEventForType('tiktok.chat');
  lastEvent.data = { ...(lastEvent.data as JsonObject), customLiveOnly: { deep: 'value' } };
  const suggestions = getTemplateSuggestions('tiktok.chat', 'en', lastEvent);
  const values = suggestions.map((entry) => entry.value);
  expect(values).toContain('event.data.customLiveOnly.deep');
  expect(values.filter((value) => value === 'event.data.comment')).toHaveLength(1);
});

test('text scope offers raw comment and Text Intelligence TTS text', () => {
  // The automation TTS text TemplateField resolves this scope: both the raw
  // comment and the TTS-normalized view must be suggestible.
  for (const suggestions of [
    getTemplateSuggestions('tiktok.chat', 'en', undefined, 'text'),
    getTemplateSuggestions(undefined, 'en', undefined, 'text'),
  ]) {
    expect(suggestions.some((entry) => entry.value === 'event.data.comment')).toBe(true);
    expect(suggestions.some((entry) => entry.value === 'event.intel.comment.tts.text')).toBe(true);
  }
});

describe('plugin availability gating', () => {
  afterEach(() => {
    resetAutocompleteRegistry();
  });

  test('textintel paths disappear from template autocomplete while disabled', () => {
    syncAutocompletePluginStates([{ id: 'textintel', installed: true, enabled: false }]);
    const values = getTemplateSuggestions('tiktok.chat', 'en').map((entry) => entry.value);
    expect(values.some((value) => value.startsWith('event.intel.comment.'))).toBe(false);
    expect(values.some((value) => value.startsWith('event.intel.user.'))).toBe(false);
    expect(values).not.toContain('event.intel.providers.textintel.comment.moderation.blocked');
    // Core paths and the host-stamped processing status survive.
    expect(values).toContain('event.data.comment');
    expect(values).toContain('event.intel.processing.status');
  });

  test('moderation verdict is suggested for chat while enabled', () => {
    const blocked = 'event.intel.providers.textintel.comment.moderation.blocked';
    const chat = getTemplateSuggestions('tiktok.chat', 'en').map((entry) => entry.value);
    expect(chat).toContain(blocked);
    const gift = getTemplateSuggestions('tiktok.gift', 'en').map((entry) => entry.value);
    expect(gift).not.toContain(blocked);
    const union = getTemplateSuggestions(undefined, 'en').map((entry) => entry.value);
    expect(union).toContain(blocked);
  });

  test('stale live-only provider paths stop being suggested once disabled', () => {
    const lastEvent = sampleEventForType('tiktok.chat');
    const intel = (lastEvent as unknown as JsonObject)['intel'] as JsonObject;
    intel['providers'] = { textintel: { comment: { moderation: { blocked: true } } } };
    const enabled = getTemplateSuggestions('tiktok.chat', 'en', lastEvent).map((entry) => entry.value);
    expect(enabled).toContain('event.intel.providers.textintel.comment');
    syncAutocompletePluginStates([{ id: 'textintel', installed: true, enabled: false }]);
    const disabled = getTemplateSuggestions('tiktok.chat', 'en', lastEvent).map((entry) => entry.value);
    expect(disabled.some((value) => value.startsWith('event.intel.providers.textintel'))).toBe(false);
    expect(disabled.some((value) => value.startsWith('event.intel.comment.'))).toBe(false);
    expect(disabled).toContain('event.data.comment');
  });
});
