import { expect, test } from 'bun:test';

import { parseBinding, settingsPathSegments } from './bindings.ts';

test('parses settings, local, and source bindings', () => {
  expect(parseBinding('settings.serverUrl')).toEqual({
    raw: 'settings.serverUrl',
    scope: 'settings',
    path: 'serverUrl',
  });
  expect(parseBinding('settings.a.b.c')).toEqual({
    raw: 'settings.a.b.c',
    scope: 'settings',
    path: 'a.b.c',
  });
  expect(parseBinding('local.testText')).toEqual({
    raw: 'local.testText',
    scope: 'local',
    path: 'testText',
  });
  expect(parseBinding('source.voices')).toEqual({
    raw: 'source.voices',
    scope: 'source',
    path: 'voices',
  });
  expect(settingsPathSegments(parseBinding('settings.a.b')!)).toEqual(['a', 'b']);
});

test('rejects expressions, unknown scopes, and prototype paths', () => {
  expect(parseBinding(undefined)).toBeUndefined();
  expect(parseBinding('')).toBeUndefined();
  expect(parseBinding('settings')).toBeUndefined();
  expect(parseBinding('settings.')).toBeUndefined();
  expect(parseBinding('eval(alert(1))')).toBeUndefined();
  expect(parseBinding('settings.a;drop')).toBeUndefined();
  expect(parseBinding('settings.__proto__.x')).toBeUndefined();
  expect(parseBinding('settings.a.prototype')).toBeUndefined();
  expect(parseBinding('window.location')).toBeUndefined();
  expect(parseBinding('local.a.b')).toBeUndefined();
  expect(parseBinding('source.a.b')).toBeUndefined();
  expect(parseBinding('settings.1abc')).toBeUndefined();
  expect(parseBinding('x'.repeat(200))).toBeUndefined();
});
