import { afterEach, describe, expect, it, vi } from 'vitest';
import { createControlClient } from './control-client.ts';

type MockIpc = {
  sent: string[];
  postMessage: (message: string) => void;
};

type HostWindow = Pick<
  Window,
  'ipc' | '__webview_on_message__' | '__tiktools_receive_batch__' | '__tiktools_host_message_queue__'
>;

function stubWindow(properties: Partial<HostWindow> = {}): HostWindow {
  const target: HostWindow = { ...properties };
  vi.stubGlobal('window', target);
  return target;
}

function installMockIpc(): MockIpc {
  const ipc: MockIpc = {
    sent: [],
    postMessage(message) {
      ipc.sent.push(message);
    },
  };
  stubWindow({ ipc });
  return ipc;
}

function hostWindow(): HostWindow {
  return window as HostWindow;
}

function sentId(): string {
  const ipc = hostWindow().ipc as MockIpc;
  return (JSON.parse(ipc.sent.at(-1)!) as { id: string }).id;
}

function hostMessage(): (raw: string) => void {
  const receive = hostWindow().__webview_on_message__;
  expect(receive).toBeTypeOf('function');
  return receive!;
}

function respond(id: string, payload: { result?: unknown; error?: unknown }): void {
  hostMessage()(JSON.stringify({ type: 'rpc-response', response: { id, ...payload } }));
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('createControlClient', () => {
  describe('call', () => {
    it('resolves with the result of the matching rpc-response', async () => {
      const ipc = installMockIpc();
      const control = createControlClient();
      control.attach();
      const pending = control.call<{ ok: boolean }>('system.ping', {});
      expect(JSON.parse(ipc.sent[0]!)).toMatchObject({
        jsonrpc: '2.0',
        id: expect.any(String),
        method: 'system.ping',
      });
      respond(sentId(), { result: { ok: true } });
      await expect(pending).resolves.toEqual({ ok: true });
      control.detach();
    });

    it('rejects with the host error code and message', async () => {
      installMockIpc();
      const control = createControlClient();
      control.attach();
      const pending = control.call('plugins.get', { pluginId: 'missing' });
      respond(sentId(), { error: { code: 'plugin_not_found', message: 'nope' } });
      await expect(pending).rejects.toMatchObject({ code: 'plugin_not_found', message: 'nope' });
      control.detach();
    });

    it('rejects without a native bridge', async () => {
      stubWindow();
      const control = createControlClient();
      control.attach();
      await expect(control.call('system.ping')).rejects.toMatchObject({ code: 'transport' });
      control.detach();
    });

    it('rejects oversized payloads before postMessage', async () => {
      const ipc = installMockIpc();
      const control = createControlClient({ timeoutMs: 5 });
      control.attach();
      const huge = 'x'.repeat(1024 * 1024 + 1);
      await expect(control.call('system.ping', { blob: huge })).rejects.toMatchObject({ code: 'too_large' });
      expect(ipc.sent).toHaveLength(0);
      control.detach();
    });

    it('rejects the pending call when the host reports too_large with its id', async () => {
      installMockIpc();
      const control = createControlClient();
      control.attach();
      const pending = control.call('system.ping', {});
      respond(sentId(), { error: { code: 'too_large', message: 'request too large' } });
      await expect(pending).rejects.toMatchObject({ code: 'too_large' });
      control.detach();
    });

    it('times out and reports a transport error', async () => {
      installMockIpc();
      const control = createControlClient({ timeoutMs: 5 });
      const transportErrors: string[] = [];
      control.onTransportError((message) => transportErrors.push(message));
      control.attach();
      await expect(control.call('system.ping')).rejects.toMatchObject({ code: 'timeout' });
      expect(transportErrors).toHaveLength(1);
      control.detach();
    });

    it('bounds pending admission and releases every slot on detach', async () => {
      const ipc = installMockIpc();
      const control = createControlClient();
      control.attach();
      const outcomes = Array.from({ length: 128 }, () =>
        control.call('system.ping').catch((error: unknown) => error),
      );
      await expect(control.call('system.ping')).rejects.toMatchObject({ code: 'overloaded' });
      expect(ipc.sent).toHaveLength(128);
      expect(control.diagnostics?.()).toMatchObject({
        pending: 128,
        capacity: 128,
        overloaded: 1,
        topicCount: 0,
        topicSubscriptions: 0,
        pushSubscriptions: 0,
      });
      control.detach();
      expect(await Promise.all(outcomes)).toHaveLength(128);
      expect(control.diagnostics?.().pending).toBe(0);
      await expect(control.call('system.ping')).rejects.toMatchObject({ code: 'transport' });
    });

    it('ignores a late response from a previous client', async () => {
      installMockIpc();
      const previous = createControlClient();
      previous.attach();
      const first = previous.call('system.ping').catch(() => undefined);
      const previousId = sentId();
      previous.detach();
      await first;
      const fresh = createControlClient();
      fresh.attach();
      const second = fresh.call('system.ping');
      const freshId = sentId();
      expect(freshId).not.toBe(previousId);
      respond(previousId, { result: 'stale' });
      expect(fresh.diagnostics?.().pending).toBe(1);
      respond(freshId, { result: 'current' });
      await expect(second).resolves.toBe('current');
      fresh.detach();
    });

    it('releases the slot on owner cancellation and ignores a late reply', async () => {
      const ipc = installMockIpc();
      const control = createControlClient();
      control.attach();
      const owner = new AbortController();
      const pending = control.call('system.ping', {}, { signal: owner.signal });
      const id = sentId();
      owner.abort();
      await expect(pending).rejects.toMatchObject({ code: 'cancelled' });
      expect(control.diagnostics?.().pending).toBe(0);
      respond(id, { result: 'late' });
      await expect(control.call('system.ping', {}, { signal: owner.signal })).rejects.toMatchObject({
        code: 'cancelled',
      });
      expect(ipc.sent).toHaveLength(1);
      control.detach();
    });
  });

  describe('inbound dispatch', () => {
    it('dispatches event notifications by topic only', () => {
      installMockIpc();
      const control = createControlClient();
      control.attach();
      const seen: unknown[] = [];
      const unsubscribe = control.onTopic('points.changed', (data) => seen.push(data));
      const receive = hostMessage();
      receive(JSON.stringify({ method: 'event', params: { topic: 'points.changed', data: { uniqueId: 'alice' } } }));
      receive(JSON.stringify({ method: 'event', params: { topic: 'live.event', data: {} } }));
      expect(seen).toEqual([{ uniqueId: 'alice' }]);
      unsubscribe();
      receive(JSON.stringify({ method: 'event', params: { topic: 'points.changed', data: { uniqueId: 'bob' } } }));
      expect(seen).toEqual([{ uniqueId: 'alice' }]);
      control.detach();
    });

    it('dispatches gaps only to gap handlers with the lost count', () => {
      installMockIpc();
      const control = createControlClient();
      control.attach();
      const seen: number[] = [];
      const topics: unknown[] = [];
      const unsubscribe = control.onGap((lost) => seen.push(lost));
      control.onTopic('points.changed', (data) => topics.push(data));
      const receive = hostMessage();
      receive(JSON.stringify({ jsonrpc: '2.0', method: 'event.gap', params: { lost: 12, resync: true } }));
      expect(seen).toEqual([12]);
      expect(topics).toEqual([]);
      receive(JSON.stringify({ method: 'event', params: { topic: 'points.changed', data: { uniqueId: 'alice' } } }));
      expect(topics).toEqual([{ uniqueId: 'alice' }]);
      receive(JSON.stringify({ method: 'event.gap', params: { resync: true } }));
      expect(seen).toEqual([12, 0]);
      unsubscribe();
      receive(JSON.stringify({ method: 'event.gap', params: { lost: 3, resync: true } }));
      expect(seen).toEqual([12, 0]);
      control.detach();
    });

    it('dispatches legacy pushes by message type', () => {
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
    });
  });

  describe('lifecycle', () => {
    it('drains the queued host messages on attach', async () => {
      installMockIpc();
      const target = hostWindow();
      target.__tiktools_host_message_queue__ = [
        JSON.stringify({ type: 'rpc-response', response: { id: 7, result: { ok: true } } }),
      ];
      const control = createControlClient();
      const pending = control.call('system.ping');
      control.attach();
      expect(target.__tiktools_host_message_queue__).toEqual([]);
      control.detach();
      await expect(pending).rejects.toMatchObject({ code: 'transport' });
    });

    it('rejects pending calls on detach', async () => {
      installMockIpc();
      const control = createControlClient();
      control.attach();
      const pending = control.call('system.ping');
      control.detach();
      await expect(pending).rejects.toMatchObject({ code: 'transport' });
    });
  });

  describe('presentation', () => {
    const view = { feed: true, points: true, behavior: false, visible: true };

    it('rejects messages from a previous revision', async () => {
      const ipc = installMockIpc();
      const control = createControlClient();
      control.attach();
      const seen: unknown[] = [];
      control.onTopic('live.ui-event', (data) => seen.push(data));
      const configuring = control.setPresentation?.(view);
      const first = JSON.parse(ipc.sent.at(-1)!) as { id: string; params: { revision: number } };
      respond(first.id, { result: { version: 1, revision: 1 } });
      await configuring;
      const hidden = control.setPresentation?.({ ...view, feed: false, points: false });
      const second = JSON.parse(ipc.sent.at(-1)!) as { id: string };
      hostMessage()(
        JSON.stringify({
          method: 'event',
          params: { topic: 'live.ui-event', revision: first.params.revision, data: 'stale' },
        }),
      );
      expect(seen).toEqual([]);
      respond(second.id, { result: {} });
      await hidden;
      control.detach();
    });

    it('drops batches from a retired page generation', async () => {
      const ipc = installMockIpc();
      const control = createControlClient();
      control.attach();
      const events: unknown[] = [];
      const pushes: unknown[] = [];
      control.onTopic('live.ui-event', (data) => events.push(data));
      control.onPush('leaderboard', (data) => pushes.push(data));
      const configuring = control.setPresentation?.(view);
      respond(sentId(), { result: { generation: 2, revision: 1 } });
      await configuring;
      const receive = hostWindow().__tiktools_receive_batch__!;
      const stale = JSON.stringify({
        method: 'event',
        params: { topic: 'live.ui-event', generation: 1, revision: 1, data: 'stale' },
      });
      receive([stale, JSON.stringify({ type: 'leaderboard', viewers: [] })], 1);
      expect(events).toEqual([]);
      expect(pushes).toEqual([]);
      receive(
        [
          JSON.stringify({
            method: 'event',
            params: { topic: 'live.ui-event', generation: 2, revision: 1, data: 'current' },
          }),
        ],
        2,
      );
      expect(events).toEqual(['current']);
      expect(ipc.sent).toHaveLength(1);
      control.detach();
    });

    it('renegotiates once above a racing previous bootstrap revision', async () => {
      const ipc = installMockIpc();
      const control = createControlClient();
      control.attach();
      const configuring = control.setPresentation?.(view);
      respond(sentId(), {
        error: { code: 'stale', message: 'old bootstrap raced', data: { revision: 10, generation: 2 } },
      });
      await Promise.resolve();
      await Promise.resolve();
      expect(ipc.sent).toHaveLength(2);
      expect(JSON.parse(ipc.sent[1]!)).toMatchObject({ params: { revision: 11 } });
      respond(sentId(), { result: { generation: 2, revision: 11 } });
      await configuring;
      const ping = control.call('system.ping');
      expect(JSON.parse(ipc.sent.at(-1)!)).toMatchObject({ pageGeneration: 2 });
      respond(sentId(), { result: {} });
      await ping;
      control.detach();
    });
  });
});

it('uses the browser transport without native globals and resynchronizes after reconnect', async () => {
  stubWindow();
  let receive: (raw: string) => void = () => {};
  let disconnect: (reason: string) => void = () => {};
  let reconnect: () => void = () => {};
  const sent: string[] = [];
  const detached = vi.fn();
  const control = createControlClient({ transport: {
    attach(onMessage, onDisconnect, onReconnect) { receive = onMessage; disconnect = onDisconnect; reconnect = onReconnect; },
    send(raw) { sent.push(raw); }, detach: detached,
  } });
  const gap = vi.fn(); control.onGap(gap); control.attach();
  expect(control.setPresentation).toBeUndefined();
  const pending = control.call('system.info');
  const id = (JSON.parse(sent[0]!) as { id: string }).id;
  receive(JSON.stringify({ type: 'rpc-response', response: { id, result: { platform: 'AndroidTermux' } } }));
  await expect(pending).resolves.toEqual({ platform: 'AndroidTermux' });
  const mutation = control.call('globals.set', { key: 'test', value: 'value' });
  const rejection = expect(mutation).rejects.toMatchObject({ code: 'transport' });
  disconnect('connection lost'); await rejection;
  reconnect(); expect(gap).toHaveBeenCalledWith(0);
  expect(sent).toHaveLength(2);
  control.detach(); expect(detached).toHaveBeenCalledOnce();
});
