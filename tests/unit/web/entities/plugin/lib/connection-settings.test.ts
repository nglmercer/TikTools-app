import { expect, it } from 'vitest';

import { SECRET_PLACEHOLDER } from '../../../../../../src/automation/plugins/declarative.ts';
import { withSchemaDefaults } from '@/shared/ui/schema-form/lib/schema-fields.ts';
import {
  connectionSummaryRows,
  echoConfirmsSave,
  echoNeedsResave,
  findServerUrlKey,
  secretSettingKeys,
  settingsMatch,
  shouldShowSummary,
} from '../../../../../../src/web/entities/plugin/lib/connection-settings.ts';

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

it('summary shows only when connected, settled, and untouched', () => {
  const connected = { editing: false, dirty: false, connectionOk: true, hasSettings: true };
  expect(shouldShowSummary(connected)).toBe(true);
  expect(shouldShowSummary({ ...connected, editing: true })).toBe(false);
  expect(shouldShowSummary({ ...connected, dirty: true })).toBe(false);
  expect(shouldShowSummary({ ...connected, connectionOk: false })).toBe(false);
  expect(shouldShowSummary({ ...connected, hasSettings: false })).toBe(false);
});

it('summary rows skip the url, secrets, and empties in schema order', () => {
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
    (value) => (typeof value === 'string' ? value : ''),
  );
  expect(findServerUrlKey(schema)).toBe('serverUrl');
  expect(rows).toEqual([
    { key: 'defaultVoice', label: 'Default voice', value: 'M1' },
    { key: 'defaultLanguage', label: 'Default language', value: 'en' },
  ]);
  const sparse = connectionSummaryRows(
    { serverUrl: 'http://localhost:17842', apiToken: 'tok-typed', defaultVoice: '' },
    schema,
    uiHints,
    findServerUrlKey(schema),
    (value) => (typeof value === 'string' ? value : ''),
  );
  expect(sparse).toEqual([]);
});

it('secret fields round-trip through the redacted placeholder', () => {
  expect(secretSettingKeys(schema, uiHints)).toEqual(['apiToken']);
  expect(
    settingsMatch(
      { serverUrl: 'http://x', apiToken: 'tok-typed' },
      { serverUrl: 'http://x', apiToken: SECRET_PLACEHOLDER },
      ['apiToken'],
    ),
  ).toBe(true);
  expect(
    settingsMatch(
      { serverUrl: 'http://x', apiToken: '' },
      { serverUrl: 'http://x', apiToken: SECRET_PLACEHOLDER },
      ['apiToken'],
    ),
  ).toBe(false);
  expect(
    settingsMatch({ defaultVoice: 'M1' }, { defaultVoice: 'F2' }, ['apiToken']),
  ).toBe(false);
});

it('host echo confirms saves without ever carrying the token', () => {
  const sent = { serverUrl: 'http://x', apiToken: 'tok-typed' };
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
  expect(
    echoNeedsResave(
      { serverUrl: 'http://x', apiToken: 'tok-typed' },
      { serverUrl: 'http://x', apiToken: SECRET_PLACEHOLDER },
      schema,
      ['apiToken'],
    ),
  ).toBe(false);
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

it('schema defaults fill display gaps but never secrets', () => {
  expect(withSchemaDefaults({}, schema)).toEqual({
    serverUrl: 'http://localhost:17842',
    defaultLanguage: 'en',
  });
  expect(withSchemaDefaults({ defaultLanguage: 'es' }, schema)).toEqual({
    serverUrl: 'http://localhost:17842',
    defaultLanguage: 'es',
  });
});
