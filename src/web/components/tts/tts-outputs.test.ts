import { expect, test } from 'bun:test';

import {
  isNotFoundOptionError,
  outputOptions,
  outputsCardState,
  outputsTarget,
} from './tts-outputs.ts';

test('outputs target derives from the option source', () => {
  expect(outputsTarget('plugin-action-options:sonicboom.server.set-output-device:device')).toEqual({
    actionType: 'sonicboom.server.set-output-device',
    field: 'device',
  });
  expect(outputsTarget('voices')).toBeUndefined();
  expect(outputsTarget('plugin-action-options:no-field')).toBeUndefined();
});

test('only 404 option errors read as unsupported endpoints', () => {
  expect(isNotFoundOptionError('HTTP 404 from 127.0.0.1')).toBe(true);
  expect(isNotFoundOptionError('HTTP 404 request failed: nope')).toBe(true);
  expect(isNotFoundOptionError('  HTTP 404 from host  ')).toBe(true);
  expect(isNotFoundOptionError('HTTP 4040 from host')).toBe(false);
  expect(isNotFoundOptionError('HTTP 500 from 127.0.0.1')).toBe(false);
  expect(isNotFoundOptionError('connection refused')).toBe(false);
  expect(isNotFoundOptionError('')).toBe(false);
  expect(isNotFoundOptionError(undefined)).toBe(false);
});

test('output rows keep an unlisted live value visible', () => {
  const outputs = [
    { value: 'default', label: 'System Default' },
    { value: 'cable', label: '' },
  ];
  expect(outputOptions(outputs, 'cable')).toEqual([
    { value: 'default', label: 'System Default' },
    { value: 'cable', label: 'cable' },
  ]);
  // A disappeared device stays on screen instead of silently resetting.
  expect(outputOptions(outputs, 'ghost')).toEqual([
    { value: 'ghost', label: 'ghost' },
    { value: 'default', label: 'System Default' },
    { value: 'cable', label: 'cable' },
  ]);
});

test('outputs card states degrade without hiding the panel', () => {
  expect(outputsCardState({ supported: false, outputs: undefined, outputsError: undefined })).toEqual({
    kind: 'hidden',
  });
  expect(outputsCardState({ supported: true, outputs: undefined, outputsError: undefined })).toEqual({
    kind: 'loading',
  });
  expect(
    outputsCardState({ supported: true, outputs: [], outputsError: 'HTTP 404 from 127.0.0.1' }),
  ).toEqual({ kind: 'unavailable', unsupported: true });
  expect(
    outputsCardState({ supported: true, outputs: [], outputsError: 'HTTP 500 from 127.0.0.1' }),
  ).toEqual({ kind: 'unavailable', unsupported: false });
  expect(outputsCardState({ supported: true, outputs: [], outputsError: undefined })).toEqual({
    kind: 'empty',
  });
  const ready = outputsCardState({
    supported: true,
    outputs: [{ value: 'default', label: 'System Default' }],
    outputsError: undefined,
  });
  expect(ready).toEqual({ kind: 'ready' });
});
