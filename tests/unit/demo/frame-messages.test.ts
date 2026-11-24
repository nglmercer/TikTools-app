import { expect, it } from 'vitest';
import { isDemoFrameState, parseFrameCommand } from '../../../src/web/demo/frame-messages.ts';
it('bounds frame state before accepting it into the document controls', () => {
  const state = { type: 'tiktools.demo.state', instance: 'docs-frame-1', ready: true, paused: true, busy: false, events: 9, status: 'paused', connectionCase: 'normal' };
  expect(isDemoFrameState(state)).toBe(true);
  for (const override of [{ events: 301 }, { events: -1 }, { events: 1.5 }, { instance: '../other' }, { ready: 'true' }, { status: 'unexpected' }, { connectionCase: 'cookie' }]) expect(isDemoFrameState({ ...state, ...override })).toBe(false);
  for (const value of [null, [], 'state']) expect(isDemoFrameState(value)).toBe(false);
});
it('limits commands to this instance and the explicit playback/recovery actions', () => {
  const command = { type: 'tiktools.demo.command', instance: 'frame-1', action: 'step' };
  expect(parseFrameCommand(command, 'frame-1')).toEqual({ action: 'step' });
  expect(parseFrameCommand(command, 'frame-2')).toBeNull();
  expect(parseFrameCommand({ ...command, action: 'shell.execute' }, 'frame-1')).toBeNull();
  expect(parseFrameCommand({ ...command, type: 'rpc-request' }, 'frame-1')).toBeNull();
  expect(parseFrameCommand({ ...command, action: 'connection-case', value: 'auth' }, 'frame-1')).toEqual({ action: 'connection-case', value: 'auth' });
  expect(parseFrameCommand({ ...command, action: 'connection-case', value: 'invalid' }, 'frame-1')).toBeNull();
});
it('accepts only the two supported theme values over the frame bridge', () => {
  const command = { type: 'tiktools.demo.command', instance: 'frame-1', action: 'theme' };
  expect(parseFrameCommand({ ...command, value: 'light' }, 'frame-1')).toEqual({ action: 'theme', value: 'light' });
  expect(parseFrameCommand({ ...command, value: 'dark' }, 'frame-1')).toEqual({ action: 'theme', value: 'dark' });
  expect(parseFrameCommand({ ...command, value: 'system' }, 'frame-1')).toBeNull();
  expect(parseFrameCommand({ ...command }, 'frame-1')).toBeNull();
});
