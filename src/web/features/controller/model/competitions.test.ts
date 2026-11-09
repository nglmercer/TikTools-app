import { ref } from 'vue';
import { expect, it, vi } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createCompetitionModel } from './competitions.ts';
import type { ControllerStatus } from './controller.ts';

function status(): ControllerStatus {
  return { epoch:'1', viewRevision:'1', phase:'ready', profile:'desk', origin:'https://host.example', errorCode:null,
    hello:{ connectionId:'connection', grantRevision:'1', hostId:'host', hostInstanceId:'instance', principalId:'moderator', sessionId:'device',
      capabilities:['competitions.read'], features:{ competitionSummary:true }, resourceRestrictions:{ competition:['alpha','charlie'] } } };
}
const row = (id: string) => ({ id, name:`Competition ${id}`, kind:'poll', lifecycle:'active', generation:'9007199254740993', deadlineMs:null, remainingMs:null });
const page = (rows: (ReturnType<typeof row> & { revision?: string })[], nextAfter: string | null = null) => ({ epoch:'1', connectionId:'connection', competitions:rows, nextAfter });

it('paginates authorized status without sending mutations', async () => {
  const current = ref(status());
  const { client, calls } = createFakeControlClient((method, params) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) :
    params?.['after'] ? page([row('charlie')]) : page([row('alpha')], 'alpha'));
  const model = createCompetitionModel(client, { status:current, generation:() => 1, stopped:() => false });
  await model.refresh(current.value, 1);
  expect(model.rows.value.map((r) => r.id)).toEqual(['alpha']);
  await model.next();
  expect(model.rows.value.map((r) => r.id)).toEqual(['charlie']);
  await model.previous();
  expect(model.rows.value[0]?.generation).toBe('9007199254740993');
  expect(calls.filter((call) => call.method === 'controller.competitions').map((call) => call.params?.['after'])).toEqual([null, 'alpha', null]);
  expect(calls.every((call) => ['controller.competitions', 'controller.status'].includes(call.method))).toBe(true);
});

it('requires canonical revisions when the host advertises them', async () => {
  const current = ref(status()); current.value.hello!.features['competitionSummaryVersions'] = true;
  let revision = '2';
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : page([{...row('alpha'), revision}]));
  const model = createCompetitionModel(client, { status:current, generation:() => 1, stopped:() => false });
  vi.useFakeTimers();
  try {
    await model.refresh(current.value, 1); expect(model.rows.value[0]?.revision).toBe('2');
    revision = '02'; vi.advanceTimersByTime(2000);
    await model.refresh(current.value, 1); expect(model.rows.value).toEqual([]);
  } finally { vi.useRealTimers(); }
});

it('sends only the visible revision and generation with a reusable grant', async () => {
  const current = ref(status());
  current.value.hello!.features['competitionSummaryVersions'] = true;
  current.value.hello!.features['competitionStop'] = true;
  current.value.hello!.capabilities.push('competitions.manage','commands.read');
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : page([{...row('alpha'),revision:'2'}]));
  const send = vi.fn(async () => true);
  const model = createCompetitionModel(client, { status:current, generation:() => 1, stopped:() => false, blocked:() => false, journalMatches:() => true, send });
  await model.refresh(current.value, 1);
  const shown = model.rows.value[0]!;
  expect(model.canStop(shown, 'pause-competition')).toBe(true);
  await model.stopCompetition(shown, 'pause-competition');
  expect(send).toHaveBeenCalledExactlyOnceWith('controller.competition.command', expect.objectContaining({id:'alpha',expectedRevision:'2',expectedGeneration:'9007199254740993',operation:'pause-competition'}));
  current.value.hello!.capabilities = ['competitions.read'];
  expect(model.canStop(shown, 'cancel-competition')).toBe(false);
});

it('never displays delayed rows after scope change or a denied LIVE restriction', async () => {
  const current = ref(status()); let release!: (value: unknown) => void;
  const { client, calls } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) :
    new Promise((resolve) => { release = resolve; }));
  const model = createCompetitionModel(client, { status:current, generation:() => 1, stopped:() => false });
  const pending = model.refresh(current.value, 1);
  current.value.hello!.resourceRestrictions = { competition:['charlie'] };
  current.value.hello!.grantRevision = '2';
  release(page([row('alpha')])); await pending;
  expect(model.rows.value).toEqual([]);
  current.value.hello!.resourceRestrictions = { live:['creator'], competition:['charlie'] };
  await model.refresh(current.value, 1);
  expect(model.allowed.value).toBe(false);
  expect(calls.filter((call) => call.method === 'controller.competitions')).toHaveLength(1);
});

it('coalesces live invalidations during a pending page and discards its stale reply', async () => {
  const current = ref(status()); let release!: (value: unknown) => void;
  let reads = 0;
  const { client } = createFakeControlClient((method) => {
    if (method === 'controller.status') return JSON.parse(JSON.stringify(current.value));
    reads += 1;
    return reads === 1 ? new Promise((resolve) => { release = resolve; }) : page([{ ...row('alpha'), name:'Updated' }]);
  });
  const model = createCompetitionModel(client, { status:current, generation:() => 1, stopped:() => false });
  const pending = model.refresh(current.value, 1);
  model.invalidate(); await model.refresh(current.value, 1);
  model.invalidate(); await model.refresh(current.value, 1);
  expect(reads).toBe(1);
  release(page([row('alpha')])); await pending;
  await vi.waitFor(() => expect(model.rows.value[0]?.name).toBe('Updated'));
  expect(reads).toBe(2);
});

it('rejects hidden IDs, malformed pages and noncanonical counters', async () => {
  const current = ref(status());
  const responses = [page([row('hidden')]), page([{ ...row('alpha'), generation:'01' }]), page([row('charlie'), row('alpha')])];
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : responses.shift());
  const model = createCompetitionModel(client, { status:current, generation:() => 1, stopped:() => false });
  vi.useFakeTimers();
  try {
    for (let i = 0; i < 3; i += 1) {
      await model.refresh(current.value, 1);
      expect(model.rows.value).toEqual([]); expect(model.error.value).toBe('Competition status unavailable');
      vi.advanceTimersByTime(2000);
    }
  } finally { vi.useRealTimers(); }
});

it('accepts host byte ordering for non-BMP IDs', async () => {
  const current = ref(status()); current.value.hello!.resourceRestrictions = {};
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : page([row('\uE000'), row('\u{10000}')]));
  const model = createCompetitionModel(client, { status:current, generation:() => 1, stopped:() => false });
  await model.refresh(current.value, 1);
  expect(model.rows.value.map((item) => item.id)).toEqual(['\uE000', '\u{10000}']);
});
