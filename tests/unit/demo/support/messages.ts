import { frameMessageType } from '../../../../src/web/demo/contract.ts';

/** A valid `tiktools.demo.state` message; overrides replace only the fields a test is probing. */
export function stateMessage(overrides: Record<string, unknown> = {}) {
  return {
    type: frameMessageType.state,
    instance: 'frame-1',
    ready: true,
    paused: true,
    busy: false,
    events: 9,
    status: 'paused',
    connectionCase: 'normal',
    ...overrides,
  };
}

/** A valid `tiktools.demo.command` message for `action`; extra fields such as `value` go in `overrides`. */
export function commandMessage(action: string, overrides: Record<string, unknown> = {}) {
  return { type: frameMessageType.command, instance: 'frame-1', action, ...overrides };
}
