import { describe, expect, test } from 'vitest';
import type { ControlClient, RpcParams, TopicHandler } from '@/shared/api/control-client.ts';
import type { ConnectionSnapshot } from '../../../../automation/contracts/generated/automation-events.ts';
import { createPlatformConnections } from './platform-connections.ts';

function setup() {
  const calls: { method: string; params?: RpcParams }[] = [];
  let receive: TopicHandler<unknown> | undefined;
  const snapshot: ConnectionSnapshot = { connectionId: 'twitch-1', platform: 'twitch', target: 'alice', state: 'connecting', channel: null, capabilities: { chat: true, follows: false, subscriptions: true, gifts: true, likes: false, shares: false, joins: false, roomStats: false, discovery: false, sendChat: false, moderation: false }, reconnectAttempt: 0, retryAfterMs: null, error: null };
  const client = {
    call: async <T,>(method: string, params?: RpcParams): Promise<T> => {
      calls.push({ method, params });
      return (method === 'live.platform.credentials.store' ? { credentialRef: 'opaque' } : method === 'live.platform.connect' ? snapshot : []) as T;
    },
    onTopic: (_topic: string, handler: TopicHandler<unknown>) => { receive = handler; return () => { receive = undefined; }; },
  } as ControlClient;
  return { calls, snapshot, state: createPlatformConnections(client), receive: (connection: ConnectionSnapshot) => receive?.({ connection }) };
}

describe('platform connections', () => {
  test('uses a credential reference, clears temporary storage, and isolates disconnects', async () => {
    const { state, calls, snapshot, receive } = setup();
    await state.connect('twitch', ' alice ', 'private-token');
    expect(calls.map((c) => c.method)).toEqual(['live.platform.credentials.store', 'live.platform.connect', 'live.platform.credentials.delete']);
    expect(calls[1]?.params).toEqual({ platform: 'twitch', target: 'alice', credentialRef: 'opaque', roomId: null });
    receive({ ...snapshot, connectionId: 'twitch-2', target: 'bob' });
    await state.disconnect('twitch-1');
    receive({ ...snapshot, state: 'disconnected' });
    expect(state.connections.value.map((c) => c.target)).toEqual(['bob']);
    state.dispose();
  });
  test('updates matching state and releases topic subscriptions', async () => {
    const { state, snapshot, receive } = setup();
    receive(snapshot);
    receive({ ...snapshot, state: 'connected' });
    expect(state.connections.value).toHaveLength(1);
    expect(state.connections.value[0]?.state).toBe('connected');
    state.dispose();
    receive({ ...snapshot, state: 'error' });
    expect(state.connections.value[0]?.state).toBe('connected');
  });
});
