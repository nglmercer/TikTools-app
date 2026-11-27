import { expect, it } from 'vitest';
import { isDemoFrameState, parseFrameCommand } from '../../../src/web/demo/frame-messages.ts';
import { commandMessage, stateMessage } from './support/messages.ts';
import { FIXTURE_ID } from '../../shared/index.ts';

it('bounds frame state before accepting it into the document controls', () => {
  const state = stateMessage({ instance: 'docs-frame-1' });
  expect(isDemoFrameState(state)).toBe(true);
  for (const override of [
    { events: 301 },
    { events: -1 },
    { events: 1.5 },
    { instance: '../other' },
    { ready: 'true' },
    { status: 'unexpected' },
    { connectionCase: 'cookie' },
  ]) expect(isDemoFrameState({ ...state, ...override })).toBe(false);
  for (const value of [null, [], 'state']) expect(isDemoFrameState(value)).toBe(false);
});
it('limits commands to this instance and the explicit playback/recovery actions', () => {
  const command = commandMessage('step');
  expect(parseFrameCommand(command, FIXTURE_ID.frame1)).toEqual({ action: 'step' });
  expect(parseFrameCommand(command, 'frame-2')).toBeNull();
  expect(parseFrameCommand({ ...command, action: 'shell.execute' }, FIXTURE_ID.frame1)).toBeNull();
  expect(parseFrameCommand({ ...command, type: 'rpc-request' }, FIXTURE_ID.frame1)).toBeNull();
  expect(parseFrameCommand(commandMessage('connection-case', { value: 'auth' }), FIXTURE_ID.frame1)).toEqual({ action: 'connection-case', value: 'auth' });
  expect(parseFrameCommand(commandMessage('connection-case', { value: 'invalid' }), FIXTURE_ID.frame1)).toBeNull();
});
it('accepts only the two supported theme values over the frame bridge', () => {
  expect(parseFrameCommand(commandMessage('theme', { value: 'light' }), FIXTURE_ID.frame1)).toEqual({ action: 'theme', value: 'light' });
  expect(parseFrameCommand(commandMessage('theme', { value: 'dark' }), FIXTURE_ID.frame1)).toEqual({ action: 'theme', value: 'dark' });
  expect(parseFrameCommand(commandMessage('theme', { value: 'system' }), FIXTURE_ID.frame1)).toBeNull();
  expect(parseFrameCommand(commandMessage('theme'), FIXTURE_ID.frame1)).toBeNull();
});
