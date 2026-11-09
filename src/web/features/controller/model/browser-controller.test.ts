import { expect, test } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { ControlCallError, type ControlCallOptions } from '@/shared/api/control-client.ts';
import type { RemoteBrowserHello, RemoteBrowserTransport } from '@/shared/api/remote-browser-transport.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createBrowserControllerClient } from './browser-controller.ts';
import type { ControllerStatus } from './controller.ts';
import ControllerRoot from '../ui/ControllerRoot.vue';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { i18n } from '@/shared/i18n/i18n.ts';

const initial: RemoteBrowserHello = {
  principalId: 'principal', sessionId: 'session', grantRevision: '3', hostId: 'host', hostInstanceId: 'instance', connectionId: 'connection',
  frameBytes: 4096, paramsBytes: 2048, requestsPerSecond: 10, requestBurst: 10, pendingCalls: 4, serverTime: 1,
  capabilities: ['workflows.read', 'workflows.toggle', 'workflows.edit', 'live.read'], subscriptionDomains: ['workflows'],
  features: { subscriptions: true, workflowDisable: true, workflowDelete: true }, resourceRestrictions: { workflow: ['welcome'] },
};
const version = (revision: string) => ({ kind: 'workflow', scope: 'host', id: 'welcome', revision });
const event = (n: string) => ({ topic: 'workflow.changed', subscriptionId: 'sub', hostInstanceId: 'instance', sequence: n,
  eventId: `event-${n}`, resource: version('2'), actor: { principalId: 'owner' }, data: { id: 'welcome' } });
function fixture(read: (options?: ControlCallOptions) => unknown = (options) => {
  options?.onMetadata?.({ resourceVersions: [version('2')] });
  return { id: 'welcome', name: 'Updated', enabled: false };
}, negotiated: RemoteBrowserHello = initial) {
  let active: RemoteBrowserHello | undefined;
  const events = new Set<(event: unknown) => void>();
  const losses = new Set<(reason: string) => void>();
  const transport = {
    connect: async () => { active = { ...negotiated }; return active; },
    currentHello: () => active,
    verifySession: async () => active !== undefined,
    disconnect: () => { if (active) { active = undefined; for (const fn of losses) fn('lost'); } },
    onReliableEvent: (fn: (event: unknown) => void) => { events.add(fn); return () => events.delete(fn); },
    onConnectionLost: (fn: (reason: string) => void) => { losses.add(fn); return () => losses.delete(fn); },
  } as unknown as RemoteBrowserTransport;
  const { client: remote, calls } = createFakeControlClient((method, _params, options) => {
    if (method === 'collaboration.subscribe') return { subscriptionId: 'sub', hostInstanceId: 'instance', grantRevision: '3', domains: ['workflows'] };
    if (method === 'collaboration.snapshot') return { subscriptionId: 'sub', snapshotId: 'snapshot', hostInstanceId: 'instance', grantRevision: '3', throughSequence: '0', domains: ['workflows'], workflows: [{ id: 'welcome', name: 'Original', enabled: true }], resourceVersions: [version('1')], nextPageToken: null };
    if (method === 'workflows.get') return read(options);
    if (method === 'live.status') return { connected: true, uniqueId: 'creator', roomId: 'room', connectionId: 'live-connection', native: true };
    if (method === 'ui.feed.browser') return { oldestCursor: '9007199254740993', latestCursor: '9007199254740993', nextCursor: '9007199254740993', gap: true, truncated: false, events: [] };
    if (method === 'ui.feed.creator') return { creatorId: 'creator', streamId: 'a'.repeat(32), oldestCursor: '0', latestCursor: '0', nextCursor: '0', gap: false, truncated: false, events: [] };
    throw new Error(`Unexpected network method: ${method}`);
  });
  const client = createBrowserControllerClient(remote, transport, 'https://host.test');
  return { client, calls, transport, emit: (value: unknown) => { for (const fn of events) fn(value); }, lose: () => transport.disconnect() };
}
async function connected(f: ReturnType<typeof fixture>) {
  f.client.attach();
  await f.client.call('controller.connect', { profile: 'browser' });
  return f.client.call<{ epoch: string; viewRevision: string; phase: string; hello: { capabilities: string[]; features: Record<string, boolean> } }>('controller.status');
}
const flush = async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); };

test('exposes the authorized workflow baseline, fences pages and denies unsupported commands', async () => {
  const f = fixture(); const status = await connected(f);
  expect(status.phase).toBe('ready');
  expect(status.hello.capabilities).toEqual(['workflows.read', 'live.read']);
  expect(status.hello.features['workflowDisable']).toBe(false);
  const page = await f.client.call<{ rows: Array<{ graph: { name: string } }>; next: number | null }>('controller.workflows', { epoch: status.epoch, viewRevision: status.viewRevision, offset: 0 });
  expect(page.rows[0]?.graph.name).toBe('Original'); expect(page.next).toBeNull();
  await expect(f.client.call('controller.workflow.command', {})).rejects.toMatchObject({ code: 'unsupported_remote' });
  f.emit(event('1')); await flush();
  const updated = await f.client.call<typeof status>('controller.status');
  expect(updated.viewRevision).not.toBe(status.viewRevision);
  await expect(f.client.call('controller.workflows', { epoch: status.epoch, viewRevision: status.viewRevision, offset: 0 })).rejects.toMatchObject({ code: 'stale_view' });
  const current = await f.client.call<{ rows: Array<{ graph: { name: string } }> }>('controller.workflows', { epoch: updated.epoch, viewRevision: updated.viewRevision, offset: 0 });
  expect(current.rows[0]?.graph.name).toBe('Updated');
  expect(f.calls.map((call) => call.method)).toEqual(['collaboration.subscribe', 'collaboration.snapshot', 'workflows.get']);
  f.client.detach();
});

test('validates tombstones and invalidates the view on a reliable event gap', async () => {
  const f = fixture(() => { throw new ControlCallError('workflow_not_found', 'not found', { resourceVersions: [version('2')] }); });
  await connected(f); f.emit(event('1')); await flush();
  const updated = await f.client.call<ControllerStatus>('controller.status');
  const page = await f.client.call<{ rows: unknown[] }>('controller.workflows', { epoch: updated.epoch, viewRevision: updated.viewRevision, offset: 0 });
  expect(page.rows).toEqual([]);
  f.emit(event('3')); await flush();
  expect((await f.client.call<ControllerStatus>('controller.status')).phase).toBe('failed');
  await expect(f.client.call('controller.workflows', { epoch: updated.epoch, viewRevision: updated.viewRevision, offset: 0 })).rejects.toMatchObject({ code: 'stale_view' });
  f.client.detach();
});

test('late reads cannot restore an old session or strand a new session update', async () => {
  let finish!: (value: unknown) => void;
  const pending = new Promise<unknown>((resolve) => { finish = resolve; });
  let calls = 0; let oldOptions: ControlCallOptions | undefined;
  const f = fixture((options) => {
    calls += 1;
    if (calls === 1) { oldOptions = options; return pending; }
    options?.onMetadata?.({ resourceVersions: [version('2')] });
    return { id: 'welcome', name: 'New session update', enabled: false };
  });
  await connected(f); f.emit(event('1')); await flush();
  await f.client.call('controller.disconnect');
  await connected(f); f.emit(event('1')); await flush();
  oldOptions?.onMetadata?.({ resourceVersions: [version('2')] });
  finish({ id: 'welcome', name: 'Stale response', enabled: false }); await flush();
  const current = await f.client.call<ControllerStatus>('controller.status');
  const page = await f.client.call<{ rows: Array<{ graph: { name: string } }> }>('controller.workflows', { epoch: current.epoch, viewRevision: current.viewRevision, offset: 0 });
  expect(page.rows[0]?.graph.name).toBe('New session update');
  expect(calls).toBe(2);
  f.client.detach();
});

test('malformed remote revisions clear the view instead of applying them', async () => {
  const f = fixture((options) => { options?.onMetadata?.({ resourceVersions: [version('0')] }); return { id: 'welcome' }; });
  await connected(f); f.emit(event('1')); await flush();
  expect((await f.client.call<{ phase: string }>('controller.status')).phase).toBe('failed');
  f.client.detach();
});

test('a changed browser session blocks publication of a fetched update', async () => {
  const f = fixture(); let checks = 0;
  f.transport.verifySession = async () => ++checks === 1;
  const status = await connected(f);
  f.emit(event('1')); await flush();
  expect((await f.client.call<ControllerStatus>('controller.status')).phase).toBe('failed');
  await expect(f.client.call('controller.workflows', { epoch: status.epoch, viewRevision: status.viewRevision, offset: 0 })).rejects.toMatchObject({ code: 'stale_view' });
  f.client.detach();
});

test('LIVE-only observers connect without a workflow subscription and receive a scoped status', async () => {
  const f = fixture(undefined, { ...initial, capabilities: ['live.read'], subscriptionDomains: [], resourceRestrictions: { live: ['creator'] } });
  const status = await connected(f);
  const result = await f.client.call('controller.live.status', { epoch: status.epoch, connectionId: 'connection' });
  expect(result).toEqual({ epoch: status.epoch, connectionId: 'connection', connected: true, uniqueId: 'creator', native: true });
  expect(f.calls.map((call) => call.method)).toEqual(['live.status']);
  await expect(f.client.call('controller.live.status', { epoch: status.epoch, connectionId: 'stale' })).rejects.toMatchObject({ code: 'stale_view' });
  f.client.detach();
});

test('LIVE status rejects an unauthorized creator and a replaced browser session', async () => {
  const restricted = fixture(undefined, { ...initial, capabilities: ['live.read'], resourceRestrictions: { live: ['other'] } });
  const status = await connected(restricted);
  await expect(restricted.client.call('controller.live.status', { epoch: status.epoch, connectionId: 'connection' })).rejects.toMatchObject({ code: 'forbidden' });
  restricted.client.detach();
  const changed = fixture(undefined, { ...initial, capabilities: ['live.read'], resourceRestrictions: {} });
  const current = await connected(changed);
  changed.transport.verifySession = async () => false;
  await expect(changed.client.call('controller.live.status', { epoch: current.epoch, connectionId: 'connection' })).rejects.toMatchObject({ code: 'stale_view' });
  changed.client.detach();
});

test('browser feed preserves cursor strings and enforces creator restrictions', async () => {
  const features = { ...initial.features, browserFeed: true, creatorFeed: true };
  const f = fixture(undefined, { ...initial, capabilities: ['live.read'], features, resourceRestrictions: {} });
  const status = await connected(f);
  const page = await f.client.call<{ latestCursor: string }>('controller.live.feed', { epoch: status.epoch, connectionId: 'connection', after: '9007199254740993' });
  expect(page.latestCursor).toBe('9007199254740993');
  expect(f.calls.at(-1)?.params).toEqual({ after: '9007199254740993' });
  f.client.detach();
  const restricted = fixture(undefined, { ...initial, capabilities: ['live.read'], features, resourceRestrictions: { live: ['creator'] } });
  const current = await connected(restricted);
  const base = { epoch: current.epoch, connectionId: 'connection', after: '0' };
  await expect(restricted.client.call('controller.live.feed', base)).rejects.toMatchObject({ code: 'forbidden' });
  await expect(restricted.client.call('controller.live.feed', { ...base, creatorId: 'other', streamId: null })).rejects.toMatchObject({ code: 'forbidden' });
  const scoped = await restricted.client.call<{ creatorId: string }>('controller.live.feed', { ...base, creatorId: 'creator', streamId: null });
  expect(scoped.creatorId).toBe('creator');
  restricted.client.detach();
});

test('browser controller screen displays the scoped view and clears it on disconnect', async () => {
  i18n.global.locale.value = 'en';
  const f = fixture();
  const wrapper = mount(ControllerRoot, { props: { browserMode: true }, global: { plugins: [i18n], provide: { [controlClientKey as symbol]: f.client } } });
  try {
    await flushPromises();
    expect(wrapper.text()).toContain('Original');
    expect(wrapper.text()).toContain('The web view supports workflow and LIVE activity reads according to your permissions.');
    expect(wrapper.text()).not.toContain('Saved profiles');
    expect(wrapper.text()).not.toContain('Connect to another PC');
    expect(wrapper.text()).not.toContain('Disable workflow');
    await wrapper.findAll('button').find((button) => button.text().includes('Original'))!.trigger('click');
    expect(wrapper.get('textarea').attributes('readonly')).toBeDefined();
    expect(wrapper.text()).toContain('Workflow details');
    f.emit(event('1')); await flushPromises();
    expect(wrapper.text()).toContain('Updated');
    f.lose(); await flushPromises();
    expect(wrapper.text()).not.toContain('Updated');
    expect(wrapper.text()).toContain('Connection failed');
  } finally { wrapper.unmount(); }
});
