import { expect, test } from 'bun:test';

import {
  isNotFoundOptionError,
  outputOptions,
  outputsCardState,
  outputsTarget,
} from './source.ts';

test('outputs target derives the switch action from the source id', () => {
  expect(outputsTarget('plugin-action-options:sonicboom.server.set-output-device:device')).toEqual({
    actionType: 'sonicboom.server.set-output-device',
    field: 'device',
  });
  expect(outputsTarget('voices')).toBeUndefined();
  expect(outputsTarget('plugin-action-options:no-field')).toBeUndefined();
});

test('only 404 option errors mean unsupported audio api', () => {
  expect(isNotFoundOptionError('HTTP 404 Not Found')).toBe(true);
  expect(isNotFoundOptionError('  HTTP 404')).toBe(true);
  expect(isNotFoundOptionError('HTTP 500 boom')).toBe(false);
  expect(isNotFoundOptionError('connection refused')).toBe(false);
  expect(isNotFoundOptionError(undefined)).toBe(false);
  expect(isNotFoundOptionError('')).toBe(false);
});

test('output rows keep the live value when the server no longer lists it', () => {
  const listed = [
    { value: 'default', label: 'System Default' },
    { value: 'Speakers', label: 'Speakers' },
  ];
  expect(outputOptions(listed, 'Speakers')).toEqual([
    { value: 'default', label: 'System Default' },
    { value: 'Speakers', label: 'Speakers' },
  ]);
  // A missing explicit selection still renders so the selector never goes blank.
  expect(outputOptions(listed, 'Unplugged USB')).toEqual([
    { value: 'Unplugged USB', label: 'Unplugged USB' },
    { value: 'default', label: 'System Default' },
    { value: 'Speakers', label: 'Speakers' },
  ]);
  expect(outputOptions(listed, '')).toHaveLength(2);
});

test('outputs card state hides the selector but never the panel', () => {
  expect(
    outputsCardState({ supported: false, outputs: undefined, outputsError: undefined }),
  ).toEqual({ kind: 'hidden' });
  expect(outputsCardState({ supported: true, outputs: undefined, outputsError: undefined })).toEqual(
    { kind: 'loading' },
  );
  expect(
    outputsCardState({ supported: true, outputs: undefined, outputsError: 'HTTP 404 gone' }),
  ).toEqual({ kind: 'unavailable', unsupported: true });
  expect(
    outputsCardState({ supported: true, outputs: [], outputsError: 'HTTP 500 boom' }),
  ).toEqual({ kind: 'unavailable', unsupported: false });
  expect(outputsCardState({ supported: true, outputs: [], outputsError: undefined })).toEqual({
    kind: 'empty',
  });
  expect(
    outputsCardState({
      supported: true,
      outputs: [{ value: 'default', label: 'System Default' }],
      outputsError: undefined,
    }),
  ).toEqual({ kind: 'ready' });
});
