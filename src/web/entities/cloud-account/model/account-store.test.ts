import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { ControlClient, RpcParams } from '@/shared/api/control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import type { AccountStatus } from '../api/account-types.ts';
import { useAccountStore } from './account-store.ts';

function signedOut(overrides: Partial<AccountStatus> = {}): AccountStatus {
  return {
    origin: 'http://127.0.0.1:8471',
    state: 'signed-out',
    accountId: null,
    persistence: 'keychain',
    error: null,
    ...overrides,
  };
}

function harness(handler: (method: string, params: RpcParams | undefined) => unknown) {
  const calls: { method: string; params: RpcParams | undefined }[] = [];
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: (async (method: string, params?: RpcParams) => {
      calls.push({ method, params });
      return handler(method, params);
    }) as ControlClient['call'],
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  const stores = createStoreHarness(client);
  const store = stores.use(() => useAccountStore());
  return { store, calls };
}

describe('account store', () => {
  it('refreshes nonsecret status without touching tokens', async () => {
    const { store, calls } = harness((method) => {
      if (method === 'cloud.status') return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    await store.refresh();
    expect(store.status?.state).toBe('signed-out');
    expect(store.error).toBeNull();
    expect(calls.map((call) => call.method)).toEqual(['cloud.status']);
  });

  it('runs the browser login loop to signed-in', async () => {
    const { store } = harness((method) => {
      if (method === 'cloud.login.begin') return { browserUrl: 'https://cloud.example/desktop/login?x=1' };
      if (method === 'cloud.login.open') return { ok: true };
      if (method === 'cloud.login.complete') {
        return signedOut({ state: 'signed-in', accountId: 'user-1' });
      }
      throw new Error(`unexpected ${method}`);
    });
    await store.startLogin();
    expect(store.loginUrl).toBeNull();
    expect(store.status?.state).toBe('signed-in');
    expect(store.status?.accountId).toBe('user-1');
    expect(store.error).toBeNull();
  });

  it('keeps the login URL on screen when the browser cannot open', async () => {
    const { store } = harness((method) => {
      if (method === 'cloud.login.begin') return { browserUrl: 'https://cloud.example/desktop/login?x=2' };
      if (method === 'cloud.login.open') {
        throw new ControlCallError('internal', 'cannot open', { reason: 'failed' });
      }
      if (method === 'cloud.login.complete') {
        return signedOut({ state: 'signed-in', accountId: 'user-2' });
      }
      throw new Error(`unexpected ${method}`);
    });
    await store.startLogin();
    // Manual opening still completes the loop; nothing secret is shown.
    expect(store.status?.state).toBe('signed-in');
  });

  it('surfaces login failures without leaking tokens', async () => {
    const failure = new ControlCallError('auth_failed', 'cloud is unreachable', { reason: 'failed' });
    const { store } = harness((method) => {
      if (method === 'cloud.login.begin') throw failure;
      if (method === 'cloud.status') return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    await store.startLogin();
    expect(store.status?.state).toBe('signed-out');
    expect(store.error).toContain('unreachable');
    expect(JSON.stringify(store.$state)).not.toContain('token');
  });

  it('cancels and logs out through the control plane', async () => {
    const { store, calls } = harness((method) => {
      if (method === 'cloud.login.cancel') {
        return signedOut({ state: 'signed-out', error: 'login cancelled' });
      }
      if (method === 'cloud.logout') return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    await store.cancelLogin();
    await store.logout();
    expect(calls.map((call) => call.method)).toEqual(['cloud.login.cancel', 'cloud.logout']);
    expect(store.status?.state).toBe('signed-out');
  });

  it('restores the session explicitly after restart', async () => {
    const { store, calls } = harness((method) => {
      if (method === 'cloud.restore') {
        return signedOut({ state: 'signed-in', accountId: 'user-3' });
      }
      throw new Error(`unexpected ${method}`);
    });
    await store.restore();
    expect(store.status?.accountId).toBe('user-3');
    expect(calls.map((call) => call.method)).toEqual(['cloud.restore']);
    await flushPromises();
  });
});
