import { expect, test } from 'bun:test';

import type { JsonObject } from '../../automation/types.ts';
import {
  AUTOSAVE_CONFIRM_TIMEOUT_MS,
  AUTOSAVE_DEBOUNCE_MS,
  connectionSummaryRows,
  echoConfirmsSave,
  findServerUrlKey,
  isHttpUrl,
  isLoopbackUrl,
  settingsEqual,
  stableSettingsJson,
  SUMMARY_ROW_LIMIT,
  withSchemaDefaults,
} from './plugin-connection-logic.ts';

test('autosave waits out typing but confirms quickly', () => {
  expect(AUTOSAVE_DEBOUNCE_MS).toBeGreaterThanOrEqual(600);
  expect(AUTOSAVE_DEBOUNCE_MS).toBeLessThanOrEqual(1000);
  expect(AUTOSAVE_CONFIRM_TIMEOUT_MS).toBeGreaterThan(AUTOSAVE_DEBOUNCE_MS);
});

test('URL validation accepts http(s) with a host', () => {
  expect(isHttpUrl('http://localhost:3000')).toBe(true);
  expect(isHttpUrl('  https://example.com/ready ')).toBe(true);
  expect(isHttpUrl('http://127.0.0.1:3000')).toBe(true);
  expect(isHttpUrl('http://[::1]:3000')).toBe(true);
  expect(isHttpUrl('')).toBe(false);
  expect(isHttpUrl('   ')).toBe(false);
  expect(isHttpUrl('localhost:3000')).toBe(false);
  expect(isHttpUrl('ftp://example.com/x')).toBe(false);
  expect(isHttpUrl('http://')).toBe(false);
  expect(isHttpUrl('javascript:alert(1)')).toBe(false);
  expect(isHttpUrl(`http://example.com/${'x'.repeat(2048)}`)).toBe(false);
});

test('loopback detection covers localhost names and addresses', () => {
  expect(isLoopbackUrl('http://localhost:3000')).toBe(true);
  expect(isLoopbackUrl('http://LOCALHOST/')).toBe(true);
  expect(isLoopbackUrl('http://127.0.0.1:3000')).toBe(true);
  expect(isLoopbackUrl('http://127.1.2.3/')).toBe(true);
  expect(isLoopbackUrl('http://[::1]:3000/')).toBe(true);
  expect(isLoopbackUrl('https://example.com/')).toBe(false);
  expect(isLoopbackUrl('http://192.168.1.10:3000/')).toBe(false);
  expect(isLoopbackUrl('http://localhost.example.com/')).toBe(false);
  expect(isLoopbackUrl('not a url')).toBe(false);
});

test('server URL key comes from format uri, never names', () => {
  expect(findServerUrlKey(undefined)).toBeUndefined();
  expect(findServerUrlKey({ type: 'object', properties: {} })).toBeUndefined();
  expect(findServerUrlKey({
    type: 'object',
    properties: {
      serverUrl: { type: 'string' },
      endpoint: { type: 'string', format: 'uri' },
    },
  })).toBe('endpoint');
  expect(findServerUrlKey({
    type: 'object',
    properties: { port: { type: 'number', format: 'uri' } },
  })).toBeUndefined();
});

test('summary rows skip the URL, secrets, and empties', () => {
  const schema = {
    type: 'object',
    properties: {
      serverUrl: { type: 'string', format: 'uri', title: 'Server URL' },
      apiToken: { type: 'string', title: 'API token' },
      defaultVoice: { type: 'string', title: 'Default voice' },
      defaultLanguage: { type: 'string', title: 'Default language' },
      playNow: { type: 'boolean', title: 'Play now' },
      blank: { type: 'string', title: 'Blank' },
    },
  };
  const uiHints = { fields: { apiToken: { secret: true } } };
  expect(connectionSummaryRows(
    {
      serverUrl: 'http://localhost:3000',
      apiToken: 'tok-123',
      defaultVoice: 'M1',
      defaultLanguage: 'es-MX',
      playNow: true,
      blank: '   ',
    },
    schema,
    uiHints,
    'serverUrl',
    'en',
  )).toEqual([
    { key: 'defaultVoice', label: 'Default voice', value: 'M1' },
    { key: 'defaultLanguage', label: 'Default language', value: 'es-MX' },
    { key: 'playNow', label: 'Play now', value: 'true' },
  ]);
  expect(connectionSummaryRows({}, schema, uiHints, 'serverUrl', 'en')).toEqual([]);
});

test('summary rows cap long schemas', () => {
  const properties: JsonObject = {};
  const values: JsonObject = {};
  for (let index = 0; index < SUMMARY_ROW_LIMIT + 2; index += 1) {
    properties[`field${index}`] = { type: 'string', title: `Field ${index}` };
    values[`field${index}`] = `v${index}`;
  }
  const rows = connectionSummaryRows(
    values,
    { type: 'object', properties },
    undefined,
    undefined,
    'en',
  );
  expect(rows).toHaveLength(SUMMARY_ROW_LIMIT);
  expect(rows[0]).toEqual({ key: 'field0', label: 'Field 0', value: 'v0' });
});

test('schema defaults fill display gaps but never secrets', () => {
  const schema = {
    type: 'object',
    properties: {
      serverUrl: { type: 'string', default: 'http://localhost:3000' },
      retries: { type: 'number', default: 3 },
      playNow: { type: 'boolean', default: false },
      nested: { type: 'object', default: { ignored: true } },
      apiToken: { type: 'string', secret: true, default: 'must-not-apply' },
    },
  };
  expect(withSchemaDefaults({}, schema)).toEqual({
    serverUrl: 'http://localhost:3000',
    retries: 3,
    playNow: false,
  });
  expect(withSchemaDefaults({ serverUrl: 'http://x/' }, schema)).toEqual({
    serverUrl: 'http://x/',
    retries: 3,
    playNow: false,
  });
  expect(withSchemaDefaults({}, undefined)).toEqual({});
});

test('save confirmation tolerates host-added defaults', () => {
  expect(echoConfirmsSave(
    { serverUrl: 'http://x/', defaultLanguage: 'en' },
    { serverUrl: 'http://x/' },
  )).toBe(true);
  expect(echoConfirmsSave(
    { serverUrl: 'http://x/' },
    { serverUrl: 'http://x/', defaultVoice: 'M1' },
  )).toBe(false);
  expect(echoConfirmsSave(
    { serverUrl: 'http://y/' },
    { serverUrl: 'http://x/' },
  )).toBe(false);
});

test('settings equality ignores key order', () => {
  expect(settingsEqual({ a: '1', b: 2 }, { b: 2, a: '1' })).toBe(true);
  expect(settingsEqual({ a: '1' }, { a: '1', b: 2 })).toBe(false);
  expect(settingsEqual({ a: '1' }, { a: '2' })).toBe(false);
  expect(stableSettingsJson({ b: 2, a: '1' })).toBe(stableSettingsJson({ a: '1', b: 2 }));
});
