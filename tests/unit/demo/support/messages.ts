import { frameMessageType } from '../../../../src/web/demo/contract.ts';
import { FIXTURE_ID } from '../../../shared/index.ts';

/** A valid `tiktools.demo.state` message; overrides replace only the fields a test is probing. */
export function stateMessage(overrides: Record<string, unknown> = {}) {
  return {
    type: frameMessageType.state,
    instance: FIXTURE_ID.frame1,
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
  return { type: frameMessageType.command, instance: FIXTURE_ID.frame1, action, ...overrides };
}
