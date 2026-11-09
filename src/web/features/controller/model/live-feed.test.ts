import { ref } from 'vue';
import { expect, it, vi } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createLiveFeedModel } from './live-feed.ts';
import type { ControllerStatus } from './controller.ts';

function status(): ControllerStatus {
  return { epoch:'1', viewRevision:'1', phase:'ready', profile:'desk', origin:'https://host.example', errorCode:null,
    hello:{ connectionId:'connection-1', grantRevision:'1', hostId:'host', hostInstanceId:'instance', principalId:'moderator', sessionId:'device', capabilities:['live.read'], features:{} } };
}
const row = (cursor: string) => ({ cursor, kind:'chat', author:'alice', authorLookupId:'alice', nickname:null, text:`message ${cursor}` });
const page = (events: ReturnType<typeof row>[], nextCursor: string, latestCursor = nextCursor, gap = false, truncated = false) =>
  ({ epoch:'1', connectionId:'connection-1', oldestCursor:events[0]?.cursor ?? '0', latestCursor, nextCursor, gap, truncated, events });

it('reads bounded host activity and preserves exact cursors above JavaScript safe integers', async () => {
  const current = ref(status()); const pages = [page([row('9007199254740993')], '9007199254740993'), page([row('9007199254740994')], '9007199254740994')];
  const { client, calls } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : pages.shift());
  const model = createLiveFeedModel(client, { status:current, generation:() => 1, stopped:() => false });
  vi.useFakeTimers();
  try {
    await model.refresh(current.value, 1); vi.advanceTimersByTime(2000); await model.refresh(current.value, 1);
    expect(model.events.value.map((event) => event.cursor)).toEqual(['9007199254740993','9007199254740994']);
    expect(calls.filter((c) => c.method === 'controller.live.feed')[1]?.params).toEqual({ epoch:'1', connectionId:'connection-1', after:'9007199254740993' });
  } finally { vi.useRealTimers(); }
});

it('drops delayed activity on disconnect, reconnection, grant change or creator restriction', async () => {
  for (const change of ['disconnect','reconnect','revoke','restrict'] as const) {
    const current = ref(status()); let release!: (value:unknown) => void;
    const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : new Promise((resolve) => { release = resolve; }));
    const model = createLiveFeedModel(client, { status:current, generation:() => 1, stopped:() => false });
    const pending = model.refresh(current.value, 1);
    if (change === 'disconnect') current.value.phase = 'idle';
    if (change === 'reconnect') current.value.hello!.connectionId = 'connection-2';
    if (change === 'revoke') current.value.hello!.capabilities = [];
    if (change === 'restrict') current.value.hello!.resourceRestrictions = { live:['creator'] };
    release(page([row('1')], '1')); await pending;
    expect(model.events.value).toEqual([]);
  }
  const current = ref(status()); current.value.hello!.resourceRestrictions = { live:['creator'] };
  const { client, calls } = createFakeControlClient(() => null);
  const model = createLiveFeedModel(client, { status:current, generation:() => 1, stopped:() => false });
  await model.refresh(current.value, 1); expect(model.allowed.value).toBe(false); expect(calls).toEqual([]);
});

it('signals a host restart and resumes from zero without mixing old events', async () => {
  const current = ref(status());
  const replies = [page([row('7')], '7'), page([], '0', '0', true), page([row('1')], '1')];
  const { client, calls } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : replies.shift());
  const model = createLiveFeedModel(client, { status:current, generation:() => 1, stopped:() => false });
  vi.useFakeTimers();
  try {
    await model.refresh(current.value, 1); vi.advanceTimersByTime(2000); await model.refresh(current.value, 1);
    expect(model.events.value).toEqual([]); expect(model.gap.value).toBe(true);
    vi.advanceTimersByTime(2000); await model.refresh(current.value, 1);
    expect(model.events.value.map((event) => event.cursor)).toEqual(['1']);
    expect(calls.filter((c) => c.method === 'controller.live.feed')[2]?.params?.after).toBe('0');
  } finally { vi.useRealTimers(); }
});

it('limits retained rows and refuses malformed or overflowing cursor pages', async () => {
  const current = ref(status()); let n = 0;
  const first = Array.from({ length:64 }, (_, i) => row(String(i + 1)));
  const second = Array.from({ length:64 }, (_, i) => row(String(i + 65)));
  const pages = [page(first, '64', '128', false, true), page(second, '128'), page([row('18446744073709551616')], '18446744073709551616')];
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : pages[n++]);
  const model = createLiveFeedModel(client, { status:current, generation:() => 1, stopped:() => false });
  vi.useFakeTimers();
  try {
    await model.refresh(current.value, 1); await model.refresh(current.value, 1);
    expect(model.events.value).toHaveLength(100); expect(model.events.value[0]?.cursor).toBe('29');
    vi.advanceTimersByTime(2000); await model.refresh(current.value, 1);
    expect(model.events.value).toEqual([]); expect(model.error.value).toBe('LIVE feed unavailable');
  } finally { vi.useRealTimers(); }
});

it('rejects a points lookup identity that does not exactly match the projected author', async () => {
  const current = ref(status());
  const forged = { ...row('1'), authorLookupId:'other' };
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : page([forged], '1'));
  const model = createLiveFeedModel(client, { status:current, generation:() => 1, stopped:() => false });
  await model.refresh(current.value, 1);
  expect(model.events.value).toEqual([]);
  expect(model.error.value).toBe('LIVE feed unavailable');
});

it('uses isolated creator streams and clears activity when the selected grant changes', async () => {
  const current = ref(status());
  current.value.hello!.features['creatorFeed'] = true;
  current.value.hello!.resourceRestrictions = { live:['bob', 'alice'] };
  const streams: Record<string, string> = { alice:'a'.repeat(32), bob:'b'.repeat(32) };
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'controller.status') return JSON.parse(JSON.stringify(current.value));
    const creatorId = String(params?.['creatorId']);
    return { ...page([row('1')], '1'), creatorId, streamId:streams[creatorId], events:[{ ...row('1'), text:`${creatorId} only` }] };
  });
  const model = createLiveFeedModel(client, { status:current, generation:() => 1, stopped:() => false });
  await model.refresh(current.value, 1);
  expect(model.selectedCreator.value).toBe('alice');
  expect(model.events.value.map((event) => event.text)).toEqual(['alice only']);
  expect(calls.find((call) => call.method === 'controller.live.feed')?.params).toEqual({ epoch:'1', connectionId:'connection-1', after:'0', creatorId:'alice', streamId:null });
  model.selectCreator('bob');
  expect(model.events.value).toEqual([]);
  await model.refresh(current.value, 1);
  expect(model.events.value.map((event) => event.text)).toEqual(['bob only']);
  current.value.hello!.resourceRestrictions = { live:['alice'] };
  current.value.hello!.grantRevision = '2';
  await model.refresh(current.value, 1);
  expect(model.selectedCreator.value).toBe('alice');
  expect(model.events.value.map((event) => event.text)).toEqual(['alice only']);
  current.value.hello!.capabilities = [];
  await model.refresh(current.value, 1);
  expect(model.events.value).toEqual([]);
});

it('resets a creator stream after host eviction without mixing old events', async () => {
  const current = ref(status()); current.value.hello!.features['creatorFeed'] = true;
  current.value.hello!.resourceRestrictions = { live:['creator'] };
  const replies = [
    { ...page([row('1')], '1'), creatorId:'creator', streamId:'a'.repeat(32) },
    { ...page([{ ...row('1'), text:'new stream' }], '1', '1', true), creatorId:'creator', streamId:'b'.repeat(32) },
  ];
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : replies.shift());
  const model = createLiveFeedModel(client, { status:current, generation:() => 1, stopped:() => false });
  vi.useFakeTimers();
  try {
    await model.refresh(current.value, 1); vi.advanceTimersByTime(2000); await model.refresh(current.value, 1);
    expect(model.events.value.map((event) => event.text)).toEqual(['new stream']);
    expect(model.gap.value).toBe(true);
  } finally { vi.useRealTimers(); }
});

it('drops an in-flight page when the moderator switches creators', async () => {
  const current = ref(status()); current.value.hello!.features['creatorFeed'] = true;
  current.value.hello!.resourceRestrictions = { live:['alice', 'bob'] };
  let releaseAlice!: (value: unknown) => void;
  const { client } = createFakeControlClient((method, params) => {
    if (method === 'controller.status') return JSON.parse(JSON.stringify(current.value));
    if (params?.['creatorId'] === 'alice') return new Promise((resolve) => { releaseAlice = resolve; });
    return { ...page([{ ...row('1'), text:'bob only' }], '1'), creatorId:'bob', streamId:'b'.repeat(32) };
  });
  const model = createLiveFeedModel(client, { status:current, generation:() => 1, stopped:() => false });
  const pending = model.refresh(current.value, 1);
  model.selectCreator('bob');
  await model.refresh(current.value, 1);
  releaseAlice({ ...page([{ ...row('1'), text:'alice private' }], '1'), creatorId:'alice', streamId:'a'.repeat(32) });
  await pending;
  expect(model.events.value.map((event) => event.text)).toEqual(['bob only']);
});

it('rejects scoped activity without a valid stream identity', async () => {
  const current = ref(status()); current.value.hello!.features['creatorFeed'] = true;
  current.value.hello!.resourceRestrictions = { live:['creator'] };
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) :
    { ...page([row('1')], '1'), creatorId:'creator', streamId:null });
  const model = createLiveFeedModel(client, { status:current, generation:() => 1, stopped:() => false });
  await model.refresh(current.value, 1);
  expect(model.events.value).toEqual([]);
  expect(model.error.value).toBe('LIVE feed unavailable');
});
