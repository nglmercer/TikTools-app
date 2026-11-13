import { effectScope, ref } from 'vue';
import { afterEach, expect, it, vi } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import type { ControllerStatus } from './controller.ts';
import { useControllerCloud } from './use-controller-cloud.ts';

function deferred<T>() { let resolve!: (value: T) => void; const promise = new Promise<T>((done) => { resolve = done; }); return { promise, resolve }; }
function model() {
  const status = ref<ControllerStatus>({ epoch: '1', viewRevision: '1', phase: 'idle', profile: null, origin: null, errorCode: null, hello: null });
  return { status, inventory: vi.fn(async () => {}), refresh: vi.fn(async () => {}), connect: vi.fn(async () => {}), disconnect: vi.fn(async () => { status.value = { ...status.value, phase: 'idle', epoch: '2', cloudPairing: null }; }) };
}
const signedIn = { account: { state: 'signed-in', accountId: 'controller', origin: 'https://cloud.example' }, deviceId: 'device' };
const teams = { items: [{ id: 'host', name: 'My host', role: 'moderator', clientKind: 'desktop' }] };
afterEach(() => { vi.useRealTimers(); });

it('clears the invitation before native enrollment finishes and ignores replies after unmount', async () => {
  const enrollment = deferred<unknown>(); const state = model();
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.cloud.status') return structuredClone(signedIn);
    if (method === 'controller.cloud.teams') return structuredClone(teams);
    if (method === 'controller.cloud.join') return enrollment.promise;
    throw new Error(`Unexpected call ${method}`);
  });
  const scope = effectScope(); const cloud = scope.run(() => useControllerCloud(client, state))!;
  await cloud.start(); cloud.profile.value = 'moderator'; cloud.token.value = 'private-invitation-marker';
  const request = cloud.join();
  expect(cloud.token.value).toBe('');
  expect(calls.find((call) => call.method === 'controller.cloud.join')?.params?.['token']).toBe('private-invitation-marker');
  cloud.stop(); scope.stop(); enrollment.resolve({ requestId: 'request', hostId: 'host' }); await request;
  expect(state.connect).not.toHaveBeenCalled(); expect(state.refresh).not.toHaveBeenCalled();
  expect(calls.every((call) => call.method.startsWith('controller.cloud.'))).toBe(true);
});

it('keeps a newly saved Cloud profile without replacing a later selected connection', async () => {
  const admitted = deferred<unknown>(); const state = model();
  const { client } = createFakeControlClient((method) => {
    if (method === 'controller.cloud.status') return structuredClone(signedIn);
    if (method === 'controller.cloud.teams') return structuredClone(teams);
    if (method === 'controller.cloud.profile.create') return admitted.promise;
    throw new Error(`Unexpected call ${method}`);
  });
  const scope = effectScope(); const cloud = scope.run(() => useControllerCloud(client, state))!;
  try {
    await cloud.start(); cloud.profile.value = 'moderator';
    const pending = cloud.connectTeam();
    state.status.value = { ...state.status.value, epoch: '2', phase: 'ready', profile: 'other' };
    admitted.resolve({ profile: 'moderator' }); await pending;
    expect(state.inventory).toHaveBeenCalledOnce(); expect(state.connect).not.toHaveBeenCalled();
  } finally { cloud.stop(); scope.stop(); }
});

it('resumes native pending approval and fences an approved response after selection changes', async () => {
  vi.useFakeTimers(); const admitted = deferred<unknown>(); const state = model();
  state.status.value = { ...state.status.value, phase: 'cloud-pairing', cloudPairing: { profile: 'moderator', hostId: 'host', requestId: 'request' } };
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'controller.cloud.status') return structuredClone(signedIn);
    if (method === 'controller.cloud.teams') return structuredClone(teams);
    if (method === 'controller.cloud.pair.poll') return admitted.promise;
    throw new Error(`Unexpected call ${method}`);
  });
  const scope = effectScope(); const cloud = scope.run(() => useControllerCloud(client, state))!;
  try {
    await cloud.start(); await vi.advanceTimersByTimeAsync(0);
    expect(calls.find((call) => call.method === 'controller.cloud.pair.poll')?.params).toEqual({ epoch: '1' });
    state.status.value = { ...state.status.value, epoch: '2', phase: 'ready', cloudPairing: null };
    admitted.resolve({ approved: true, profile: 'moderator' }); await vi.advanceTimersByTimeAsync(0);
    expect(state.inventory).toHaveBeenCalledOnce(); expect(state.connect).not.toHaveBeenCalled();
  } finally { cloud.stop(); scope.stop(); }
});
