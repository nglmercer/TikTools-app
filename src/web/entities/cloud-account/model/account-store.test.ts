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

  it('restores a keychain session once and shares concurrent attempts', async () => {
    const { store, calls } = harness((method) => {
      if (method === 'cloud.status') return signedOut();
      if (method === 'cloud.restore') return signedOut({ state: 'signed-in', accountId: 'user-9' });
      throw new Error(`unexpected ${method}`);
    });
    await Promise.all([store.ensureRestored(), store.ensureRestored()]);
    expect(store.status?.state).toBe('signed-in');
    expect(calls.filter((call) => call.method === 'cloud.restore')).toHaveLength(1);
  });

  it('treats "no saved session" as a quiet signed-out start', async () => {
    const { store } = harness((method) => {
      if (method === 'cloud.status') return signedOut();
      if (method === 'cloud.restore') {
        throw new ControlCallError('unavailable', 'no saved cloud session', { reason: 'no-saved-session' });
      }
      throw new Error(`unexpected ${method}`);
    });
    await store.ensureRestored();
    expect(store.status?.state).toBe('signed-out');
    expect(store.error).toBeNull();
  });

  it('surfaces an expired saved session', async () => {
    const { store } = harness((method) => {
      if (method === 'cloud.status') return signedOut();
      if (method === 'cloud.restore') {
        throw new ControlCallError('internal', 'cloud session expired; sign in again', { reason: 'expired' });
      }
      throw new Error(`unexpected ${method}`);
    });
    await store.ensureRestored();
    expect(store.error).toContain('expired');
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

  it('never lets an earlier status response undo logout', async () => {
    let release!: (value: AccountStatus) => void;
    const delayed = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store } = harness((method) => {
      if (method === 'cloud.status') return delayed;
      if (method === 'cloud.logout') return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    store.status = signedOut({ state: 'signed-in', accountId: 'alice' });
    const old = store.refresh();
    await store.logout();
    release(signedOut({ state: 'signed-in', accountId: 'alice' }));
    await old;
    expect(store.status?.state).toBe('signed-out');
    expect(store.loading).toBe(false);
    expect(store.working).toBe(false);
  });

  it('holds the loading fence through restore and rejects a reply after logout', async () => {
    let release!: (value: AccountStatus) => void;
    const delayed = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store } = harness((method) => {
      if (method === 'cloud.status') return signedOut();
      if (method === 'cloud.restore') return delayed;
      if (method === 'cloud.logout') return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    const old = store.ensureRestored();
    await flushPromises();
    expect(store.loading).toBe(true);
    await store.logout();
    release(signedOut({ state: 'signed-in', accountId: 'alice' }));
    await old;
    expect(store.status?.state).toBe('signed-out');
    expect(store.loading).toBe(false);
  });

  it('does not open or complete a login cancelled before begin replies', async () => {
    let release!: (value: { browserUrl: string }) => void;
    const delayed = new Promise<{ browserUrl: string }>((resolve) => { release = resolve; });
    const { store, calls } = harness((method) => {
      if (method === 'cloud.login.begin') return delayed;
      if (method === 'cloud.login.cancel') return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    const old = store.startLogin();
    const cancelled = store.cancelLogin();
    expect(calls.map((entry) => entry.method)).toEqual(['cloud.login.begin']);
    release({ browserUrl: 'https://cloud.example/desktop/login' });
    await Promise.all([old, cancelled]);
    expect(calls.map((entry) => entry.method)).toEqual(['cloud.login.begin', 'cloud.login.cancel']);
    expect(store.loginUrl).toBeNull();
    expect(store.status?.state).toBe('signed-out');
    expect(store.working).toBe(false);
  });

  it('does not wait for a superseded restore before a later refresh', async () => {
    let release!: (value: AccountStatus) => void;
    let requests = 0;
    const delayed = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store } = harness((method) => {
      if (method === 'cloud.status') { requests += 1; return requests === 1 ? delayed : signedOut(); }
      if (method === 'cloud.logout') return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    const old = store.ensureRestored();
    await store.logout();
    await store.refresh();
    expect(requests).toBe(2);
    release(signedOut({ state: 'signed-in', accountId: 'alice' }));
    await old;
    expect(store.status?.state).toBe('signed-out');
  });

  it('waits for restore before login and keeps the restored account', async () => {
    let release!: (value: AccountStatus) => void;
    const pending = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store, calls } = harness((method) => {
      if (method === 'cloud.status') return signedOut();
      if (method === 'cloud.restore') return pending;
      throw new Error(`unexpected ${method}`);
    });
    const restored = store.ensureRestored();
    await flushPromises();
    const login = store.startLogin();
    expect(calls.map((entry) => entry.method)).toEqual(['cloud.status', 'cloud.restore']);
    release(signedOut({ state: 'signed-in', accountId: 'alice' }));
    await Promise.all([restored, login]);
    expect(store.status?.accountId).toBe('alice');
    expect(store.working).toBe(false);
    expect(store.loading).toBe(false);
  });

  it('waits for an explicit restore before refreshing status', async () => {
    let release!: (value: AccountStatus) => void;
    const pending = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store, calls } = harness((method) => {
      if (method === 'cloud.restore') return pending;
      throw new Error(`unexpected ${method}`);
    });
    const restored = store.restore();
    const refreshed = store.refresh();
    expect(calls.map((entry) => entry.method)).toEqual(['cloud.restore']);
    release(signedOut({ state: 'signed-in', accountId: 'alice' }));
    await Promise.all([restored, refreshed]);
    expect(store.status?.accountId).toBe('alice');
  });

  it('cancels a login waiting on restore without discarding its result', async () => {
    let release!: (value: AccountStatus) => void;
    const pending = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store, calls } = harness((method) => {
      if (method === 'cloud.restore') return pending;
      throw new Error(`unexpected ${method}`);
    });
    const restored = store.restore();
    const login = store.startLogin();
    const cancelled = store.cancelLogin();
    release(signedOut({ state: 'signed-in', accountId: 'alice' }));
    await Promise.all([restored, login, cancelled]);
    expect(calls.map((entry) => entry.method)).toEqual(['cloud.restore']);
    expect(store.status?.accountId).toBe('alice');
    expect(store.working).toBe(false);
  });

  it('does not cancel a native login when begin fails', async () => {
    let reject!: (error: Error) => void;
    const pending = new Promise<{ browserUrl: string }>((_, fail) => { reject = fail; });
    const { store, calls } = harness((method) => {
      if (method === 'cloud.login.begin') return pending;
      if (method === 'cloud.status') return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    const login = store.startLogin();
    const cancelled = store.cancelLogin();
    reject(new Error('begin failed'));
    await Promise.all([login, cancelled]);
    expect(calls.map((entry) => entry.method)).toEqual(['cloud.login.begin', 'cloud.status']);
    expect(store.status?.state).toBe('signed-out');
    expect(store.working).toBe(false);
  });


  it('reconciles native state when cancellation fails', async () => {
    const { store, calls } = harness((method) => {
      if (method === 'cloud.login.cancel') throw new Error('cancel unavailable');
      if (method === 'cloud.status') return signedOut({ state: 'signing-in' });
      throw new Error(`unexpected ${method}`);
    });
    await store.cancelLogin();
    expect(calls.map((entry) => entry.method)).toEqual(['cloud.login.cancel', 'cloud.status']);
    expect(store.status?.state).toBe('signing-in');
    expect(store.error).toContain('cancel unavailable');
    expect(store.working).toBe(false);
  });

});
