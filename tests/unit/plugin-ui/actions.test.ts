import { expect, test } from 'vitest';

import type { JsonValue } from '../../../src/shared/json.ts';
import { normalizeAction } from '../../../src/plugin-ui/actions.ts';
import { PLUGIN_ID, RPC } from '../../shared/index.ts';

test('accepts every allowlisted action type', () => {
  expect(normalizeAction({ type: 'save-settings' })).toEqual({ type: 'save-settings' });
  expect(normalizeAction({ type: 'test-connection' })).toEqual({ type: 'test-connection' });
  expect(
    normalizeAction({ type: 'plugin-action', actionType: PLUGIN_ID.sonicboomServerSpeak }),
  ).toEqual({ type: 'plugin-action', actionType: PLUGIN_ID.sonicboomServerSpeak });
  expect(normalizeAction({ type: 'refresh-source', source: 'plugin-action-options:a:b' })).toEqual(
    { type: 'refresh-source', source: 'plugin-action-options:a:b' },
  );
  expect(normalizeAction({ type: 'open-media-picker', accept: 'audio/*' })).toEqual({
    type: 'open-media-picker',
    accept: 'audio/*',
  });
});

test('rejects generic RPC and malformed descriptors', () => {
  const bad: JsonValue[] = [
    undefined as unknown as JsonValue,
    null as unknown as JsonValue,
    'save-settings' as unknown as JsonValue,
    { type: 'rpc', method: RPC.appStateGet },
    { type: 'eval', code: 'alert(1)' },
    { type: 'plugin-action' },
    { type: 'plugin-action', actionType: '../../etc/passwd' },
    { type: 'plugin-action', actionType: 'ok.action', config: 'nope' },
    { type: 'refresh-source' },
    { type: 'refresh-source', source: '' },
    { type: 'open-media-picker', accept: 42 },
  ];
  for (const value of bad) expect(normalizeAction(value)).toBeUndefined();
});
