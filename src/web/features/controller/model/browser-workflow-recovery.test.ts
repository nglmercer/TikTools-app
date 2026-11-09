import { expect, test, vi } from 'vitest';
import { ControlCallError, type ControlClient } from '@/shared/api/control-client.ts';
import type { RemoteBrowserHello, RemoteBrowserTransport } from '@/shared/api/remote-browser-transport.ts';
import { createBrowserWorkflowRecovery } from './browser-workflow-recovery.ts';

const hello: RemoteBrowserHello = {
  principalId: 'principal', sessionId: 'session', grantRevision: '3', hostId: 'host', hostInstanceId: 'instance', connectionId: 'connection',
  frameBytes: 4096, paramsBytes: 2048, requestsPerSecond: 1, requestBurst: 1, pendingCalls: 4, serverTime: 1,
  capabilities: ['workflows.read'], subscriptionDomains: ['workflows'], features: { subscriptions: true }, resourceRestrictions: { workflow: ['welcome', 'second'] },
};
const subscription = { subscriptionId: 'sub', hostInstanceId: 'instance', grantRevision: '3', domains: ['workflows'] };
const ver = (id: string, revision = '1') => ({ kind: 'workflow', scope: 'host', id, revision });
const page = (id: string, nextPageToken: string | null = null) => ({
  subscriptionId: 'sub', snapshotId: 'snapshot', hostInstanceId: 'instance', grantRevision: '3', throughSequence: '0',
  domains: ['workflows'], workflows: [{ id, enabled: true }], resourceVersions: [ver(id)], nextPageToken,
});
const event = (seq: string, name = 'welcome') => ({ topic: 'workflow.changed', subscriptionId: 'sub', hostInstanceId: 'instance',
  sequence: seq, eventId: `event-${seq}`, resource: ver(name, '2'), actor: { principalId: 'owner' }, data: { id: name } });
function fixture(handler: (method: string, params: Record<string, unknown>) => Promise<unknown>) {
  let current: RemoteBrowserHello | undefined = hello;
  const events = new Set<(value: unknown) => void>();
  const losses = new Set<(reason: string) => void>();
  const transport = {
    currentHello: () => current,
    onReliableEvent: (fn: (value: unknown) => void) => { events.add(fn); return () => events.delete(fn); },
    onConnectionLost: (fn: (reason: string) => void) => { losses.add(fn); return () => losses.delete(fn); },
  } as unknown as RemoteBrowserTransport;
  const call = vi.fn(handler);
  const client = { call } as unknown as ControlClient;
  return { transport, client, call, emit: (value: unknown) => { for (const fn of events) fn(value); },
    lose: () => { current = undefined; for (const fn of losses) fn('lost'); } };
}
const tick = async () => { for (let i = 0; i < 8; i++) await Promise.resolve(); };

test('publishes a complete paged baseline and requires acknowledgements in event order', async () => {
  const f = fixture(async (method, params) => method === 'collaboration.subscribe' ? subscription :
    params['pageToken'] ? page('second') : page('welcome', 'next'));
  const changed = vi.fn();
  const recovery = createBrowserWorkflowRecovery(f.client, f.transport, hello, changed);
  await recovery.start();
  expect(recovery.baseline()?.map((row) => row.version.id)).toEqual(['welcome', 'second']);
  const copy = recovery.baseline()!; copy[0]!.graph['enabled'] = false;
  expect(recovery.baseline()![0]!.graph['enabled']).toBe(true);
  f.emit(event('1')); f.emit(event('2', 'second'));
  const first = recovery.next()!;
  expect(first.sequence).toBe('1');
  expect(() => recovery.acknowledge({ ...first, eventId: 'other' })).toThrow('acknowledgement');
  recovery.acknowledge(first);
  expect(recovery.next()?.sequence).toBe('2');
  recovery.acknowledge(recovery.next()!);
  expect(recovery.next()).toBeNull();
  expect(changed).toHaveBeenCalled();
  expect(f.call.mock.calls.map((c) => c[0])).toEqual(['collaboration.subscribe', 'collaboration.snapshot', 'collaboration.snapshot']);
  recovery.close(); expect(recovery.baseline()).toBeNull();
});

test('buffers post-snapshot events until every page is validated', async () => {
  let finish!: (result: unknown) => void;
  const pending = new Promise<unknown>((resolve) => { finish = resolve; });
  const f = fixture(async (method) => method === 'collaboration.subscribe' ? subscription : pending);
  const recovery = createBrowserWorkflowRecovery(f.client, f.transport, hello);
  const start = recovery.start(); await tick();
  f.emit(event('1'));
  expect(recovery.baseline()).toBeNull(); expect(recovery.next()).toBeNull();
  finish(page('welcome')); await start;
  expect(recovery.baseline()).toHaveLength(1); expect(recovery.next()?.sequence).toBe('1');
});

test('a disconnect during a page read cannot publish the old snapshot', async () => {
  let finish!: (result: unknown) => void;
  const pending = new Promise<unknown>((resolve) => { finish = resolve; });
  const f = fixture(async (method) => method === 'collaboration.subscribe' ? subscription : pending);
  const recovery = createBrowserWorkflowRecovery(f.client, f.transport, hello);
  const start = recovery.start(); await tick(); f.lose(); finish(page('welcome'));
  await expect(start).rejects.toThrow('invalidated');
  expect(recovery.baseline()).toBeNull(); expect(recovery.next()).toBeNull();
});

test('connection loss cancels an outstanding page call', async () => {
  const f = fixture(async () => { throw new Error('unused'); });
  let signal: AbortSignal | undefined;
  const client = { call: async (method: string, _params: unknown, options: { signal: AbortSignal }) => {
    if (method === 'collaboration.subscribe') return subscription;
    signal = options.signal;
    return new Promise((_, reject) => signal!.addEventListener('abort', () => reject(new Error('cancelled')), { once: true }));
  } } as unknown as ControlClient;
  const recovery = createBrowserWorkflowRecovery(client, f.transport, hello);
  const starting = recovery.start(); await tick(); f.lose();
  await expect(starting).rejects.toThrow('invalidated');
  expect(signal?.aborted).toBe(true);
  expect(recovery.baseline()).toBeNull();
});

test('rejects duplicate resources, stale grants, unpermitted rows and repeated page tokens', async () => {
  for (const pages of [
    [page('welcome', 'next'), page('welcome')],
    [{ ...page('welcome'), grantRevision: '4' }],
    [page('private')],
    [page('welcome', 'same'), page('second', 'same')],
    [Object.fromEntries(Object.entries(page('welcome')).filter(([key]) => key !== 'nextPageToken'))],
  ]) {
    let index = 0;
    const f = fixture(async (method) => method === 'collaboration.subscribe' ? subscription : pages[index++]);
    const recovery = createBrowserWorkflowRecovery(f.client, f.transport, hello);
    await expect(recovery.start()).rejects.toThrow();
    expect(recovery.baseline()).toBeNull();
  }
});

test('a sequence gap or event outside the grant clears the visible baseline', async () => {
  for (const bad of [event('2'), event('1', 'private'), { ...event('1'), hostInstanceId: 'other' }]) {
    const f = fixture(async (method) => method === 'collaboration.subscribe' ? subscription : page('welcome'));
    const recovery = createBrowserWorkflowRecovery(f.client, f.transport, hello);
    await recovery.start(); f.emit(bad);
    expect(recovery.baseline()).toBeNull(); expect(recovery.next()).toBeNull();
  }
});

test('only explicit pre-dispatch rate limits are retried', async () => {
  vi.useFakeTimers();
  try {
    let attempts = 0;
    const f = fixture(async (method) => {
      if (method === 'collaboration.subscribe' && attempts++ === 0) throw new ControlCallError('rate_limited', 'limited', { retryAfterMs: 1000 });
      return method === 'collaboration.subscribe' ? subscription : page('welcome');
    });
    const recovery = createBrowserWorkflowRecovery(f.client, f.transport, hello);
    const starting = recovery.start(); await tick();
    await vi.advanceTimersByTimeAsync(1000);
    await starting;
    expect(f.call.mock.calls.map((c) => c[0])).toEqual(['collaboration.subscribe', 'collaboration.subscribe', 'collaboration.snapshot']);
    recovery.close();
    const uncertain = fixture(async () => { throw new ControlCallError('transport', 'connection lost'); });
    const failed = createBrowserWorkflowRecovery(uncertain.client, uncertain.transport, hello);
    await expect(failed.start()).rejects.toMatchObject({ code: 'transport' });
    expect(uncertain.call).toHaveBeenCalledOnce();
  } finally { vi.useRealTimers(); }
});

test('an overloaded event queue invalidates rather than dropping reliable changes', async () => {
  const f = fixture(async (method) => method === 'collaboration.subscribe' ? subscription : page('welcome'));
  const recovery = createBrowserWorkflowRecovery(f.client, f.transport, hello);
  await recovery.start();
  for (let n = 1; n <= 4097; n++) f.emit(event(String(n)));
  expect(recovery.baseline()).toBeNull(); expect(recovery.next()).toBeNull();
});
