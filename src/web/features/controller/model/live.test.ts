import { ref } from 'vue';
import { expect, it, vi } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { createLiveStatusModel } from './live.ts';
import type { ControllerStatus } from './controller.ts';

function status(): ControllerStatus {
  return { epoch: '1', viewRevision: '1', phase: 'ready', profile: 'desk', origin: 'https://host.example', errorCode: null,
    hello: { connectionId: 'connection-1', grantRevision: '1', hostId: 'host', hostInstanceId: 'instance', principalId: 'moderator', sessionId: 'device', capabilities: ['live.read'], features: {} } };
}
const reply = { epoch: '1', connectionId: 'connection-1', connected: true, uniqueId: 'creator', native: true };

it('displays only bounded read-only LIVE status for the selected authorized host', async () => {
  const current = ref(status());
  const { client, calls } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : structuredClone(reply));
  const model = createLiveStatusModel(client, { status: current, generation: () => 1, stopped: () => false });
  await model.refresh(current.value, 1);
  expect(model.data.value).toEqual({ connected: true, uniqueId: 'creator', native: true });
  expect(calls.map((c) => c.method)).toEqual(['controller.live.status', 'controller.status']);
  expect(calls[0]?.params).toEqual({ epoch: '1', connectionId: 'connection-1' });
  current.value.hello!.resourceRestrictions = { live: ['other'] };
  await model.refresh(current.value, 1);
  expect(model.data.value).toBeNull();
  expect(model.error.value).toBe('LIVE status unavailable');
});

it('drops delayed responses after disconnect, reconnect or permission revocation', async () => {
  for (const change of ['disconnect', 'reconnect', 'revoke'] as const) {
    const current = ref(status()); let release!: (value: unknown) => void;
    const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : new Promise((resolve) => { release = resolve; }));
    const model = createLiveStatusModel(client, { status: current, generation: () => 1, stopped: () => false });
    const pending = model.refresh(current.value, 1);
    if (change === 'disconnect') current.value.phase = 'idle';
    if (change === 'reconnect') current.value.hello!.connectionId = 'connection-2';
    if (change === 'revoke') current.value.hello!.capabilities = [];
    release(structuredClone(reply)); await pending;
    expect(model.data.value).toBeNull();
  }
});

it('rejects unexpected fields and never carries stale connected state through a failed refresh', async () => {
  const current = ref(status()); let invalid = false;
  const { client } = createFakeControlClient((method) => method === 'controller.status' ? JSON.parse(JSON.stringify(current.value)) : invalid ? { ...reply, secret: 'not displayed' } : structuredClone(reply));
  const model = createLiveStatusModel(client, { status: current, generation: () => 1, stopped: () => false });
  await model.refresh(current.value, 1);
  expect(model.data.value?.connected).toBe(true);
  invalid = true; current.value.hello!.grantRevision = '2';
  await model.refresh(current.value, 1);
  expect(model.data.value).toBeNull(); expect(model.error.value).toBe('LIVE status unavailable');
});

it('bounds polling to one read per two seconds while retrying after failure', async () => {
  vi.useFakeTimers();
  try {
    vi.setSystemTime(100_000);
    const current = ref(status()); let fail = false;
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'controller.status') return JSON.parse(JSON.stringify(current.value));
      if (fail) throw new Error('offline');
      return structuredClone(reply);
    });
    const model = createLiveStatusModel(client, { status: current, generation: () => 1, stopped: () => false });
    await model.refresh(current.value, 1); await model.refresh(current.value, 1);
    expect(calls.filter((c) => c.method === 'controller.live.status')).toHaveLength(1);
    fail = true; vi.advanceTimersByTime(2000); await model.refresh(current.value, 1);
    expect(model.data.value).toBeNull(); expect(model.error.value).toBe('LIVE status unavailable');
    fail = false; vi.advanceTimersByTime(2000); await model.refresh(current.value, 1);
    expect(model.data.value?.connected).toBe(true);
    expect(calls.filter((c) => c.method === 'controller.live.status')).toHaveLength(3);
  } finally { vi.useRealTimers(); }
});
