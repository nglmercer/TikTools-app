import { expect, test } from 'bun:test';

import { SECRET_PLACEHOLDER } from '../../automation/plugins/declarative.ts';
import {
  connectionSummaryRows,
  echoConfirmsSave,
  echoNeedsResave,
  findServerUrlKey,
  secretSettingKeys,
  settingsMatch,
  shouldShowSummary,
  withSchemaDefaults,
} from './plugin-connection-logic.ts';

const schema = {
  type: 'object',
  properties: {
    serverUrl: { type: 'string', format: 'uri', title: 'Server URL', default: 'http://localhost:17842' },
    apiToken: { type: 'string', title: 'API token', secret: true },
    defaultVoice: { type: 'string', title: 'Default voice' },
    defaultLanguage: { type: 'string', title: 'Default language', default: 'en' },
  },
};

const uiHints = { fields: { apiToken: { secret: true } } };

test('summary shows only when connected, settled, and untouched', () => {
  const connected = { editing: false, dirty: false, connectionOk: true, hasSettings: true };
  expect(shouldShowSummary(connected)).toBe(true);
  // Pressing Edit settings reveals the complete form immediately.
  expect(shouldShowSummary({ ...connected, editing: true })).toBe(false);
  // Unsaved edits keep the form visible even with a passing probe.
  expect(shouldShowSummary({ ...connected, dirty: true })).toBe(false);
  // Failed or missing probes never collapse to the summary.
  expect(shouldShowSummary({ ...connected, connectionOk: false })).toBe(false);
  // Settings still loading shows the form skeleton, not a summary.
  expect(shouldShowSummary({ ...connected, hasSettings: false })).toBe(false);
});

test('summary rows skip the url, secrets, and empties in schema order', () => {
  const rows = connectionSummaryRows(
    {
      serverUrl: 'http://localhost:17842',
      apiToken: SECRET_PLACEHOLDER,
      defaultVoice: 'M1',
      defaultLanguage: 'en',
    },
    schema,
    uiHints,
    findServerUrlKey(schema),
    'en',
  );
  expect(findServerUrlKey(schema)).toBe('serverUrl');
  expect(rows).toEqual([
    { key: 'defaultVoice', label: 'Default voice', value: 'M1' },
    { key: 'defaultLanguage', label: 'Default language', value: 'en' },
  ]);
  // Empty values leave no row; the token never appears even when typed.
  const sparse = connectionSummaryRows(
    { serverUrl: 'http://localhost:17842', apiToken: 'tok-typed', defaultVoice: '' },
    schema,
    uiHints,
    findServerUrlKey(schema),
    'en',
  );
  expect(sparse).toEqual([]);
});

test('secret fields round-trip through the redacted placeholder', () => {
  expect(secretSettingKeys(schema, uiHints)).toEqual(['apiToken']);
  // A typed secret matches its redacted echo: the save reads clean while
  // the typed value stays in the draft for Show/Hide.
  expect(
    settingsMatch(
      { serverUrl: 'http://x', apiToken: 'tok-typed' },
      { serverUrl: 'http://x', apiToken: SECRET_PLACEHOLDER },
      ['apiToken'],
    ),
  ).toBe(true);
  // Clearing a secret stays dirty until the host confirms it.
  expect(
    settingsMatch(
      { serverUrl: 'http://x', apiToken: '' },
      { serverUrl: 'http://x', apiToken: SECRET_PLACEHOLDER },
      ['apiToken'],
    ),
  ).toBe(false);
  // Ordinary differences never match.
  expect(
    settingsMatch({ defaultVoice: 'M1' }, { defaultVoice: 'F2' }, ['apiToken']),
  ).toBe(false);
});

test('host echo confirms saves without ever carrying the token', () => {
  const sent = { serverUrl: 'http://x', apiToken: 'tok-typed' };
  // The echo carries the placeholder instead of the stored secret.
  expect(
    echoConfirmsSave(
      { serverUrl: 'http://x', apiToken: SECRET_PLACEHOLDER, defaultLanguage: 'en' },
      sent,
      ['apiToken'],
    ),
  ).toBe(true);
  expect(
    echoConfirmsSave({ serverUrl: 'http://other', apiToken: SECRET_PLACEHOLDER }, sent, ['apiToken']),
  ).toBe(false);
  // A typed secret vs its redacted echo converges: no resave loop.
  expect(
    echoNeedsResave(
      { serverUrl: 'http://x', apiToken: 'tok-typed' },
      { serverUrl: 'http://x', apiToken: SECRET_PLACEHOLDER },
      schema,
      ['apiToken'],
    ),
  ).toBe(false);
  // Genuine drift after the echo schedules another save.
  expect(
    echoNeedsResave(
      { serverUrl: 'http://x', apiToken: 'tok-newer' },
      { serverUrl: 'http://x', apiToken: SECRET_PLACEHOLDER },
      schema,
      ['apiToken'],
    ),
  ).toBe(false);
  expect(
    echoNeedsResave(
      { serverUrl: 'http://changed', apiToken: 'tok-typed' },
      { serverUrl: 'http://x', apiToken: SECRET_PLACEHOLDER },
      schema,
      ['apiToken'],
    ),
  ).toBe(true);
});

test('schema defaults fill display gaps but never secrets', () => {
  expect(withSchemaDefaults({}, schema)).toEqual({
    serverUrl: 'http://localhost:17842',
    defaultLanguage: 'en',
  });
  expect(withSchemaDefaults({ defaultLanguage: 'es' }, schema)).toEqual({
    serverUrl: 'http://localhost:17842',
    defaultLanguage: 'es',
  });
});
