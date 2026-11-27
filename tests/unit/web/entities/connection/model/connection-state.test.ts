import { afterEach, beforeEach, describe, expect, test } from 'vitest';

import type { ControlClient, RpcParams } from '@/shared/api/control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { useConnection } from '../../../../../../src/web/entities/connection/model/connection-state.ts';
import { getSavedUsername } from '../../../../../../src/web/entities/connection/lib/creator-preferences.ts';
import { FIXTURE_ID, RPC, TOPIC } from '../../../../../shared/index.ts';

type Answer = (method: string, params?: RpcParams) => unknown;
type TopicSink = (data: never) => void;

const notStubbed: Answer = () => Promise.reject(new ControlCallError('unavailable', 'not stubbed'));

function stubControl(answer: Answer = notStubbed, hooks: { unsubscribed?: () => void } = {}): ControlClient & {
  calls: Array<{
    method: string;
    params?: RpcParams;
  }>;
  emit: (topic: string, data: unknown) => void;
} {
  const calls: Array<{
    method: string;
    params?: RpcParams;
  }> = [];
  const topics = new Map<string, TopicSink>();
  const untyped: Function = answer;
  return {
    calls,
    emit: (topic, data) => {
      const sink = topics.get(topic);
      if (sink) Reflect.apply(sink, undefined, [data]);
    },
    attach: () => {},
    detach: () => {},
    call: async <T,>(method: string, params?: RpcParams): Promise<T> => {
      calls.push({ method, params });
      return Reflect.apply(untyped, undefined, [method, params]);
    },
    onTopic: (topic, handler) => {
      topics.set(topic, handler);
      return () => hooks.unsubscribed?.();
    },
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
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

describe('useConnection monitored connection', () => {
  beforeEach(() => localStorage.clear());
  afterEach(() => localStorage.clear());

  test('persists the creator and applies the feed preference without calling live.connect', () => {
    const control = stubControl();
    let feedOpens = 0;
    const connection = useConnection(control, {
      ...callbacks,
      goFeed: () => { feedOpens += 1; },
    });
    connection.setOpenFeedAfterConnect(true);
    const target = connection.prepareMonitoredConnection(' @last ');
    expect(target).toBe('last');
    expect(control.calls).toHaveLength(0);
    expect(connection.uniqueId.value).toBe('last');
    expect(getSavedUsername()).toBe('last');
    expect(connection.recents.value).toContain('last');
    expect(feedOpens).toBe(1);
  });

  test('rejects an empty creator before starting the monitor', () => {
    const control = stubControl();
    const connection = useConnection(control, callbacks);
    connection.setUniqueId('');
    expect(connection.prepareMonitoredConnection()).toBe('');
    expect(connection.error.value).toBe('connection.handleRequired');
    expect(control.calls).toHaveLength(0);
  });
});

describe('useConnection pick-live', () => {
  test('missing username feedback clears when a username is entered', () => {
    const control = stubControl();
    const connection = useConnection(control, callbacks);
    connection.setUniqueId('');
    connection.handleConnect();
    expect(connection.error.value).toBe('connection.handleRequired');
    expect(control.calls).toHaveLength(0);
    connection.setUniqueId(' @creator ');
    expect(connection.error.value).toBe('');
  });

  test('a failed pick restores the previous creator pill', async () => {
    const control = stubControl(() => Promise.reject(new ControlCallError('unavailable', 'no rooms')));
    const connection = useConnection(control, callbacks);
    const previous = connection.activeCreator.value;

    connection.handlePickLive();
    // Transient searching state while the RPC is in flight.
    expect(connection.activeCreator.value).toBe('connection.searchingRooms');
    await flushPromises();

    expect(connection.status.value).toBe('error');
    expect(connection.error.value).toBe('no rooms');
    expect(connection.activeCreator.value).toBe(previous);
  });

  test('an empty cookie is sent through for guest mode', async () => {
    const control = stubControl(() => Promise.resolve({ connected: false }));
    const connection = useConnection(control, callbacks);

    connection.handlePickLive();
    await flushPromises();

    expect(control.calls).toHaveLength(1);
    expect(control.calls[0]?.method).toBe(RPC.livePick);
    expect(control.calls[0]?.params).toEqual({ sessionCookie: '' });
    // Unconnected results also restore the pill instead of stranding it.
    expect(connection.activeCreator.value).not.toBe('connection.searchingRooms');
  });

  test('a successful pick surfaces the creator in the input', async () => {
    const control = stubControl(() =>
      Promise.resolve({
        connected: true,
        uniqueId: '@picked_creator',
        roomId: '123',
        connectionId: FIXTURE_ID.connection1,
        native: true,
      }));
    const connection = useConnection(control, callbacks);
    expect(connection.uniqueId.value).toBe('');

    connection.handlePickLive();
    await flushPromises();

    expect(connection.status.value).toBe('connecting');
    expect(connection.uniqueId.value).toBe('picked_creator');
  });

  test('a provided cookie is passed through trimmed', async () => {
    const control = stubControl(() => Promise.resolve({ connected: false }));
    const connection = useConnection(control, callbacks);
    connection.setCookie('  sessionid=abc  ');

    connection.handlePickLive();
    await flushPromises();

    expect(control.calls[0]?.params).toEqual({ sessionCookie: 'sessionid=abc' });
  });
});

describe('useConnection creator/input sync', () => {
  test('connect writes the normalized handle back to the input', async () => {
    const control = stubControl(() => Promise.resolve({
          connected: true,
          uniqueId: 'alice',
          roomId: '123',
          connectionId: FIXTURE_ID.connection1,
          native: true,
        }));
    const connection = useConnection(control, callbacks);
    connection.setUniqueId('  @alice  ');

    connection.handleConnect();
    await flushPromises();

    expect(connection.activeCreator.value).toBe('alice');
    expect(connection.uniqueId.value).toBe('alice');
  });

  test('a live.connected event syncs the pill and the input', () => {
    const control = stubControl();
    const connection = useConnection(control, callbacks);
    connection.setUniqueId('stale_typing');

    control.emit(TOPIC.liveConnected, { uniqueId: '@Canonical_Name' });

    expect(connection.status.value).toBe('connected');
    expect(connection.activeCreator.value).toBe('Canonical_Name');
    expect(connection.uniqueId.value).toBe('Canonical_Name');
  });

  test('a resync read syncs the input with the authoritative creator', async () => {
    const control = stubControl(() => Promise.resolve({
          connected: true,
          uniqueId: '@resync_creator',
          roomId: '123',
          connectionId: FIXTURE_ID.connection1,
          native: true,
        }));
    const connection = useConnection(control, callbacks);
    connection.setUniqueId('something_else');

    await connection.refreshStatus();

    expect(connection.activeCreator.value).toBe('resync_creator');
    expect(connection.uniqueId.value).toBe('resync_creator');
  });
});

describe('useConnection refreshStatus', () => {
  test('a connected read rebuilds the pill from the authoritative state', async () => {
    const control = stubControl((method) => {
      expect(method).toBe('live.status');
      return Promise.resolve({
        connected: true,
        uniqueId: '@resync_creator',
        roomId: '123',
        connectionId: FIXTURE_ID.connection1,
        native: true,
      });
    });
    const connection = useConnection(control, callbacks);

    await connection.refreshStatus();

    expect(connection.status.value).toBe('connected');
    expect(connection.activeCreator.value).toBe('resync_creator');
  });

  test('a disconnected read clears a stale pill after a gap', async () => {
    const control = stubControl(() => Promise.resolve({
          connected: false,
          uniqueId: null,
          roomId: null,
          connectionId: null,
          native: false,
        }));
    const connection = useConnection(control, callbacks);

    await connection.refreshStatus();

    expect(connection.status.value).toBe('disconnected');
  });

  test('a failed refresh keeps local state instead of guessing', async () => {
    const control = stubControl(() => Promise.reject(new ControlCallError('unavailable', 'host away')));
    const connection = useConnection(control, callbacks);

    await connection.refreshStatus();

    expect(connection.status.value).toBe('idle');
    expect(connection.error.value).toBe('');
  });
});

test('a stale failed connect cannot overwrite a later disconnect', async () => {
  let failConnect: ((error: Error) => void) | undefined;
  const control = stubControl((method) => method === RPC.liveConnect
    ? new Promise<never>((_resolve, reject) => { failConnect = reject; })
    : Promise.resolve({ connected: false }));
  const connection = useConnection(control, callbacks);
  connection.handleConnect('first');
  connection.handleDisconnect();
  failConnect?.(new Error('old failure'));
  await flushPromises();
  expect(connection.status.value).toBe('disconnected');
  expect(connection.error.value).toBe('');
  connection.dispose();
});

test('dispose fences pending status refresh and releases topic subscriptions', async () => {
  let resolveStatus: ((result: unknown) => void) | undefined;
  let removed = 0;
  const control = stubControl(
    () => new Promise<unknown>((resolve) => { resolveStatus = resolve; }),
    { unsubscribed: () => { removed += 1; } },
  );
  const connection = useConnection(control, callbacks);
  const refresh = connection.refreshStatus();
  connection.dispose();
  resolveStatus?.({ connected: true, uniqueId: 'retired' });
  await refresh;
  expect(connection.status.value).toBe('idle');
  expect(removed).toBe(4);
});


test('older status refresh cannot overwrite a newer refresh', async () => {
  const answers: Array<(result: unknown) => void> = [];
  const control = stubControl(() => new Promise<unknown>((resolve) => { answers.push(resolve); }));
  const connection = useConnection(control, callbacks);
  const old = connection.refreshStatus();
  const latest = connection.refreshStatus();
  answers[1]?.({ connected: false });
  await latest;
  answers[0]?.({ connected: true, uniqueId: 'old' });
  await old;
  expect(connection.status.value).toBe('disconnected');
  connection.dispose();
});

describe('useConnection reconnect reporting', () => {
  test('live.reconnecting surfaces the attempt and delay, and connected clears them', () => {
    const control = stubControl();
    const connection = useConnection(control, callbacks);

    control.emit('live.reconnecting', { attempt: 2, delayMs: 4000 });
    expect(connection.status.value).toBe('retrying');
    expect(connection.reconnectAttempt.value).toBe(2);
    expect(connection.reconnectDelayMs.value).toBe(4000);

    control.emit(TOPIC.liveConnected, { uniqueId: 'alice' });
    expect(connection.status.value).toBe('connected');
    expect(connection.reconnectAttempt.value).toBeNull();
    expect(connection.reconnectDelayMs.value).toBeNull();
  });

  test('a payload without numbers reports an unknown attempt, never NaN', () => {
    const control = stubControl();
    const connection = useConnection(control, callbacks);

    control.emit('live.reconnecting', {});
    expect(connection.status.value).toBe('retrying');
    expect(connection.reconnectAttempt.value).toBeNull();
    expect(connection.reconnectDelayMs.value).toBeNull();
  });
});

describe('useConnection reconnect vs creator switch', () => {
  test('a reconnect to the same creator does not reset the feed', () => {
    let resets = 0;
    const control = stubControl(() => Promise.resolve({ connected: false }));
    const connection = useConnection(control, {
      ...callbacks,
      resetFeed: () => { resets += 1; },
    });

    connection.handleConnect('alice');
    const afterFirstConnect = resets;
    connection.handleReconnect();
    expect(resets).toBe(afterFirstConnect);
    connection.dispose();
  });

  test('switching to another creator resets the feed, a repeat does not', () => {
    let resets = 0;
    const control = stubControl(() => Promise.resolve({ connected: false }));
    const connection = useConnection(control, {
      ...callbacks,
      resetFeed: () => { resets += 1; },
    });

    // Establish a baseline creator first: whether this first connect resets
    // depends on the persisted username from earlier tests.
    connection.handleConnect('bob');
    const baseline = resets;

    connection.handleConnect('bob');
    expect(resets).toBe(baseline);

    connection.handleConnect('carol');
    expect(resets).toBe(baseline + 1);
    connection.dispose();
  });
});

describe('useConnection cancellation', () => {
  test.each([RPC.liveConnect, RPC.livePick])('ignores a late result from a cancelled %s', async (method) => {
    let finish: ((value: unknown) => void) | undefined;
    const control = stubControl((requested) => requested === method
      ? new Promise<unknown>((resolve) => { finish = resolve; })
      : Promise.resolve({ connected: false }));
    const connection = useConnection(control, callbacks);
    connection.noteCreatorSeen('alice', false);
    if (method === RPC.livePick) connection.handlePickLive();
    else connection.handleConnect('alice');
    expect(connection.status.value).toBe('connecting');
    connection.handleDisconnect();
    expect(connection.status.value).toBe('disconnected');
    expect(connection.activeCreator.value).toBe('alice');
    finish?.({ connected: true, uniqueId: 'late_creator' });
    await flushPromises();
    expect(control.calls.map((call) => call.method)).toEqual([method, RPC.liveDisconnect]);
    expect(connection.status.value).toBe('disconnected');
    expect(connection.uniqueId.value).toBe('alice');
    expect(connection.error.value).toBe('');
    connection.dispose();
  });
});

describe('useConnection terminal transport failures', () => {
  test('a disconnected notification preserves the final error until recovery', () => {
    const control = stubControl();
    const connection = useConnection(control, callbacks);
    control.emit('live.error', { phase: 'live', message: 'TikTok rate limit (HTTP 429)' });
    control.emit('live.disconnected', {});
    expect(connection.status.value).toBe('error');
    expect(connection.error.value).toContain('429');
    control.emit('live.reconnecting', { attempt: 1, delayMs: 30000 });
    expect(connection.status.value).toBe('retrying');
    expect(connection.error.value).toBe('');
    control.emit(TOPIC.liveConnected, { uniqueId: 'alice' });
    expect(connection.status.value).toBe('connected');
    expect(connection.reconnectDelayMs.value).toBeNull();
  });
});

test('feed redirect is opt-in on connect and pick-live', () => {
  let redirects = 0;
  const connection = useConnection(stubControl(), { ...callbacks, goFeed: () => { redirects += 1; } });
  connection.handleConnect('creator');
  connection.handlePickLive();
  expect(redirects).toBe(0);
  connection.setOpenFeedAfterConnect(true);
  connection.handleConnect('creator');
  connection.handlePickLive();
  expect(redirects).toBe(2);
  connection.dispose();
});
