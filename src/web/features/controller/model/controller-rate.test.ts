import { expect, it, vi } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createControllerModel, type ControllerStatus } from './controller.ts';

it('shares minimum gateway capacity fairly across LIVE and competition polling', async () => {
  vi.useFakeTimers();
  const current: ControllerStatus = { epoch:'1', viewRevision:'1', phase:'ready', profile:'desk', origin:'https://host.example', errorCode:null,
    hello:{ connectionId:'connection', grantRevision:'1', hostId:'host', hostInstanceId:'instance', principalId:'moderator', sessionId:'device',
      capabilities:['live.read','competitions.read'], features:{ competitionSummary:true }, resourceRestrictions:{},
      limits:{ requestsPerSecond:1, requestBurst:1 } } };
  const scheduled: Array<{ method: string; at: number }> = [];
  let lastRequest = -Infinity;
  const consume = (method: string) => {
    const now = performance.now();
    if (now - lastRequest < 1000) throw new Error('simulated rate limit');
    lastRequest = now; scheduled.push({ method, at:now });
  };
  const { client } = createFakeControlClient((method) => {
    if (method === 'controller.profiles') return [];
    if (method === 'controller.status') return structuredClone(current);
    if (method === 'controller.workflows') return { epoch:'1', viewRevision:'1', rows:[], next:null };
    if (method === 'controller.live.status') { consume(method); return { epoch:'1', connectionId:'connection', connected:true, uniqueId:'creator', native:true }; }
    if (method === 'controller.live.feed') { consume(method); return { epoch:'1', connectionId:'connection', oldestCursor:'0', latestCursor:'0', nextCursor:'0', gap:false, truncated:false, events:[] }; }
    if (method === 'controller.competitions') { consume(method); return { epoch:'1', connectionId:'connection', competitions:[{ id:'allowed', name:'Allowed', kind:'poll', lifecycle:'active', generation:'1', deadlineMs:null, remainingMs:null }], nextAfter:null }; }
    throw new Error(method);
  });
  const model = createControllerModel(client);
  let heartbeat: ReturnType<typeof setInterval> | undefined;
  try {
    await model.start();
    heartbeat = setInterval(() => { consume('heartbeat'); }, 5000);
    await vi.advanceTimersByTimeAsync(12500);
    expect(scheduled.filter((call) => call.method.startsWith('controller.')).map((call) => call.method)).toEqual([
      'controller.live.status', 'controller.live.feed', 'controller.competitions', 'controller.live.status', 'controller.live.feed',
    ]);
    expect(model.live.data.value?.uniqueId).toBe('creator');
    expect(model.competitions.rows.value[0]?.id).toBe('allowed');
    current.hello!.resourceRestrictions = { competition:[] };
    current.hello!.grantRevision = '2';
    await model.refresh();
    expect(model.competitions.rows.value).toEqual([]);
  } finally {
    if (heartbeat) clearInterval(heartbeat);
    model.stop(); vi.useRealTimers();
  }
});
