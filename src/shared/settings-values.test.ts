import { expect, test } from 'bun:test';

import { readSettingsPath, toSettingValues, writeSettingsPath } from './settings-values.ts';

test('toSettingValues preserves nested objects, arrays, and null', () => {
  const payload = toSettingValues({
    serverUrl: 'http://127.0.0.1:17842',
    retries: 3,
    enabled: true,
    nothing: null,
    audio: { volume: 0.5, devices: ['a', 'b'], nested: { deep: [1, { two: 2 }] } },
    dropped: undefined,
  });
  expect(payload).toEqual({
    serverUrl: 'http://127.0.0.1:17842',
    retries: 3,
    enabled: true,
    nothing: null,
    audio: { volume: 0.5, devices: ['a', 'b'], nested: { deep: [1, { two: 2 }] } },
  });
  // Only undefined is dropped; nothing else is silently discarded.
  expect('dropped' in payload).toBe(false);
});

test('toSettingValues round-trips through JSON serialization', () => {
  const values = { audio: { volume: 0.5 }, tags: ['x'] };
  const roundTripped = JSON.parse(JSON.stringify(toSettingValues(values)));
  expect(roundTripped).toEqual(values);
});

test('settings paths read and write nested values immutably', () => {
  const values = { audio: { volume: 0.5 } };
  expect(readSettingsPath(values, ['audio', 'volume'])).toBe(0.5);
  expect(readSettingsPath(values, ['audio', 'missing'])).toBeUndefined();
  expect(readSettingsPath(values, ['audio', 'volume', 'deeper'])).toBeUndefined();
  expect(readSettingsPath(undefined, ['audio'])).toBeUndefined();

  const next = writeSettingsPath(values, ['audio', 'volume'], 0.75);
  expect(next).toEqual({ audio: { volume: 0.75 } });
  // The input is untouched (immutable update).
  expect(values).toEqual({ audio: { volume: 0.5 } });

  // Missing intermediate objects are created.
  expect(writeSettingsPath({}, ['a', 'b', 'c'], 1)).toEqual({ a: { b: { c: 1 } } });
  // Empty segments are a no-op.
  expect(writeSettingsPath(values, [], 1)).toBe(values);
});
