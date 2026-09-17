import { expect, test } from 'bun:test';

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
