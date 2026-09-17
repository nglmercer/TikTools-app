import { describe, expect, test } from 'bun:test';

import type { ControlClient, RpcParams } from '../platform/control-client.ts';
import { ControlCallError } from '../platform/control-client.ts';
import { useConnection } from './connection.ts';

function stubControl(overrides?: Partial<ControlClient>): ControlClient & {
  calls: Array<{ method: string; params?: RpcParams }>;
} {
  const calls: Array<{ method: string; params?: RpcParams }> = [];
  return {
    calls,
    attach: () => {},
    detach: () => {},
    call: <T,>(method: string, params?: RpcParams): Promise<T> => {
      calls.push({ method, params });
      return Promise.reject(
        new ControlCallError('unavailable', 'not stubbed'),
      ) as Promise<T>;
    },
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    ...overrides,
  };
}

const callbacks = {
  goFeed: () => {},
  resetFeed: () => {},
  translate: (key: string): string => key,
};

async function flushPromises(): Promise<void> {
  for (let i = 0; i < 5; i += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

describe('useConnection pick-live', () => {
  test('a failed pick restores the previous creator pill', async () => {
    const control = stubControl({
      call: <T,>(): Promise<T> =>
        Promise.reject(new ControlCallError('unavailable', 'no rooms')) as Promise<T>,
    });
    const connection = useConnection(control, callbacks);
    const previous = connection.activeCreator.value;

    connection.handlePickLive();
    // Transient searching state while the RPC is in flight.
    expect(connection.activeCreator.value).toBe('searchingRooms');
    await flushPromises();

    expect(connection.status.value).toBe('error');
    expect(connection.error.value).toBe('no rooms');
    expect(connection.activeCreator.value).toBe(previous);
  });

  test('an empty cookie is sent through for guest mode', async () => {
    const control = stubControl({
      call: <T,>(method: string, params?: RpcParams): Promise<T> => {
        control.calls.push({ method, params });
        return Promise.resolve({ connected: false } as T);
      },
    });
    const connection = useConnection(control, callbacks);

    connection.handlePickLive();
    await flushPromises();

    expect(control.calls).toHaveLength(1);
    expect(control.calls[0]?.method).toBe('live.pick');
    expect(control.calls[0]?.params).toEqual({ sessionCookie: '' });
    // Unconnected results also restore the pill instead of stranding it.
    expect(connection.activeCreator.value).not.toBe('searchingRooms');
  });

  test('a provided cookie is passed through trimmed', async () => {
    const control = stubControl({
      call: <T,>(method: string, params?: RpcParams): Promise<T> => {
        control.calls.push({ method, params });
        return Promise.resolve({ connected: false } as T);
      },
    });
    const connection = useConnection(control, callbacks);
    connection.setCookie('  sessionid=abc  ');

    connection.handlePickLive();
    await flushPromises();

    expect(control.calls[0]?.params).toEqual({ sessionCookie: 'sessionid=abc' });
  });
});
