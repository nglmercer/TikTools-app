import { expect, test } from 'bun:test';

import { createControlClient } from './control-client.ts';

type MockIpc = {
  sent: string[];
  postMessage: (message: string) => void;
};

function installMockIpc(): MockIpc {
  const mock: MockIpc = {
    sent: [],
    postMessage(message: string): void {
      mock.sent.push(message);
    },
  };
  (globalThis as Record<string, unknown>)['window'] = { ipc: mock };
  return mock;
}

function hostMessage(): (raw: string) => void {
  const target = (globalThis as Record<string, unknown>)['window'] as Record<string, unknown>;
  const receive = target['__webview_on_message__'] as (raw: string) => void;
  expect(receive).toBeFunction();
  return receive;
}

test('call resolves with the result of the matching rpc-response', async () => {
  installMockIpc();
  const control = createControlClient();
  control.attach();
  const pending = control.call<{ ok: boolean }>('system.ping', {});
  const mock = ((globalThis as Record<string, unknown>)['window'] as Record<string, unknown>)[
    'ipc'
  ] as MockIpc;
  expect(JSON.parse(mock.sent[0] as string)).toMatchObject({
    jsonrpc: '2.0',
    id: 1,
    method: 'system.ping',
  });
  const receive = hostMessage();
  receive(JSON.stringify({ type: 'rpc-response', response: { id: 1, result: { ok: true } } }));
  await expect(pending).resolves.toEqual({ ok: true });
  control.detach();
  delete (globalThis as Record<string, unknown>)['window'];
});

test('call rejects with the host error code and message', async () => {
  installMockIpc();
  const control = createControlClient();
  control.attach();
  const pending = control.call('plugins.get', { pluginId: 'missing' });
  hostMessage()(
    JSON.stringify({
      type: 'rpc-response',
      response: { id: 1, error: { code: 'plugin_not_found', message: 'nope' } },
    }),
  );
  await expect(pending).rejects.toMatchObject({ code: 'plugin_not_found', message: 'nope' });
  control.detach();
  delete (globalThis as Record<string, unknown>)['window'];
});

test('call rejects without a native bridge', async () => {
  (globalThis as Record<string, unknown>)['window'] = {};
  const control = createControlClient();
  control.attach();
  await expect(control.call('system.ping')).rejects.toMatchObject({ code: 'transport' });
  control.detach();
  delete (globalThis as Record<string, unknown>)['window'];
});

test('call times out and reports a transport error', async () => {
  installMockIpc();
  const control = createControlClient({ timeoutMs: 5 });
  const transportErrors: string[] = [];
  control.onTransportError((message) => transportErrors.push(message));
  control.attach();
  await expect(control.call('system.ping')).rejects.toMatchObject({ code: 'timeout' });
  expect(transportErrors.length).toBe(1);
  control.detach();
  delete (globalThis as Record<string, unknown>)['window'];
});

test('event notifications dispatch by topic only', () => {
  installMockIpc();
  const control = createControlClient();
  control.attach();
  const seen: unknown[] = [];
  const unsubscribe = control.onTopic('points.changed', (data) => seen.push(data));
  const receive = hostMessage();
  receive(
    JSON.stringify({
      method: 'event',
      params: { topic: 'points.changed', data: { uniqueId: 'alice' } },
    }),
  );
  receive(
    JSON.stringify({ method: 'event', params: { topic: 'live.event', data: {} } }),
  );
  expect(seen).toEqual([{ uniqueId: 'alice' }]);
  unsubscribe();
  receive(
    JSON.stringify({
      method: 'event',
      params: { topic: 'points.changed', data: { uniqueId: 'bob' } },
    }),
  );
  expect(seen).toEqual([{ uniqueId: 'alice' }]);
  control.detach();
  delete (globalThis as Record<string, unknown>)['window'];
});

test('legacy pushes dispatch by message type', () => {
  installMockIpc();
  const control = createControlClient();
  control.attach();
  const seen: string[] = [];
  control.onPush('leaderboard', () => seen.push('leaderboard'));
  const receive = hostMessage();
  receive(JSON.stringify({ type: 'leaderboard', viewers: [] }));
  receive(JSON.stringify({ type: 'room-stats', viewers: 0, totalUsers: 0, topViewers: [] }));
  expect(seen).toEqual(['leaderboard']);
  control.detach();
  delete (globalThis as Record<string, unknown>)['window'];
});

test('attach drains the queued host messages', async () => {
  installMockIpc();
  const target = (globalThis as Record<string, unknown>)['window'] as Record<string, unknown>;
  target['__tiktools_host_message_queue__'] = [
    JSON.stringify({ type: 'rpc-response', response: { id: 7, result: { ok: true } } }),
  ];
  const control = createControlClient();
  const pending = control.call('system.ping');
  // The queued response arrives during attach; ids will not match, so the
  // call must still be pending afterwards (no crash, no stray resolution).
  control.attach();
  expect(target['__tiktools_host_message_queue__']).toEqual([]);
  control.detach();
  await expect(pending).rejects.toMatchObject({ code: 'transport' });
  delete (globalThis as Record<string, unknown>)['window'];
});

test('detach rejects pending calls', async () => {
  installMockIpc();
  const control = createControlClient();
  control.attach();
  const pending = control.call('system.ping');
  control.detach();
  await expect(pending).rejects.toMatchObject({ code: 'transport' });
  delete (globalThis as Record<string, unknown>)['window'];
});
