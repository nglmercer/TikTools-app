import { expect, test } from 'vitest';

import type { JsonValue } from '../../../src/shared/json.ts';
import { normalizeAction } from '../../../src/plugin-ui/actions.ts';
import { NODE_TYPE, PLUGIN_ID, RPC } from '../../shared/index.ts';

test('accepts every allowlisted action type', () => {
  expect(normalizeAction({ type: 'save-settings' })).toEqual({ type: 'save-settings' });
  expect(normalizeAction({ type: 'test-connection' })).toEqual({ type: 'test-connection' });
  expect(
    normalizeAction({ type: NODE_TYPE.pluginAction, actionType: PLUGIN_ID.sonicboomServerSpeak }),
  ).toEqual({ type: NODE_TYPE.pluginAction, actionType: PLUGIN_ID.sonicboomServerSpeak });
  expect(normalizeAction({ type: NODE_TYPE.refreshSource, source: 'plugin-action-options:a:b' })).toEqual(
    { type: NODE_TYPE.refreshSource, source: 'plugin-action-options:a:b' },
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
    { type: NODE_TYPE.pluginAction },
    { type: NODE_TYPE.pluginAction, actionType: '../../etc/passwd' },
    { type: NODE_TYPE.pluginAction, actionType: 'ok.action', config: 'nope' },
    { type: NODE_TYPE.refreshSource },
    { type: NODE_TYPE.refreshSource, source: '' },
    { type: 'open-media-picker', accept: 42 },
  ];
  for (const value of bad) expect(normalizeAction(value)).toBeUndefined();
});
