import { flushPromises } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import type { ControlClient, RpcParams } from '@/shared/api/control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import { createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import type { AccountStatus } from '../../../../../../src/web/entities/cloud-account/api/account-types.ts';
import { useAccountStore } from '../../../../../../src/web/entities/cloud-account/model/account-store.ts';
import { ACCOUNT_STATE, FIXTURE_ID, RPC } from '../../../../../shared/index.ts';

function signedOut(overrides: Partial<AccountStatus> = {}): AccountStatus {
  return {
    origin: 'http://127.0.0.1:8471',
    state: ACCOUNT_STATE.signedOut,
    accountId: null,
    persistence: 'keychain',
    error: null,
    ...overrides,
  };
}

function harness(
  handler: (method: string, params: RpcParams | undefined) => unknown,
  reserve: () => unknown = () => ({ operationId: '0123456789abcdef0123456789abcdef' }),
) {
  const calls: {
    method: string;
    params: RpcParams | undefined
  }[] = [];
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: (async (method: string, params?: RpcParams) => {
      calls.push({ method, params });
      if (method === RPC.cloudLoginReserve) return reserve();
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
      if (method === RPC.cloudStatus) return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    await store.refresh();
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedOut);
    expect(store.error).toBeNull();
    expect(calls.map((call) => call.method)).toEqual([RPC.cloudStatus]);
  });

  it('restores a keychain session once and shares concurrent attempts', async () => {
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudStatus) return signedOut();
      if (method === RPC.cloudRestore) return signedOut({ state: ACCOUNT_STATE.signedIn, accountId: 'user-9' });
      throw new Error(`unexpected ${method}`);
    });
    await Promise.all([store.ensureRestored(), store.ensureRestored()]);
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedIn);
    expect(calls.filter((call) => call.method === RPC.cloudRestore)).toHaveLength(1);
  });

  it('treats "no saved session" as a quiet signed-out start', async () => {
    const { store } = harness((method) => {
      if (method === RPC.cloudStatus) return signedOut();
      if (method === RPC.cloudRestore) {
        throw new ControlCallError('unavailable', 'no saved cloud session', { reason: 'no-saved-session' });
      }
      throw new Error(`unexpected ${method}`);
    });
    await store.ensureRestored();
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedOut);
    expect(store.error).toBeNull();
  });

  it('surfaces an expired saved session', async () => {
    const { store } = harness((method) => {
      if (method === RPC.cloudStatus) return signedOut();
      if (method === RPC.cloudRestore) {
        throw new ControlCallError('internal', 'cloud session expired; sign in again', { reason: 'expired' });
      }
      throw new Error(`unexpected ${method}`);
    });
    await store.ensureRestored();
    expect(store.error).toContain('expired');
  });

  it('runs the browser login loop to signed-in', async () => {
    const { store } = harness((method) => {
      if (method === RPC.cloudLoginBegin) return { browserUrl: 'https://cloud.example/desktop/login?x=1' };
      if (method === RPC.cloudLoginOpen) return { ok: true };
      if (method === RPC.cloudLoginComplete) {
        return signedOut({ state: ACCOUNT_STATE.signedIn, accountId: FIXTURE_ID.user1 });
      }
      throw new Error(`unexpected ${method}`);
    });
    await store.startLogin();
    expect(store.loginUrl).toBeNull();
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedIn);
    expect(store.status?.accountId).toBe(FIXTURE_ID.user1);
    expect(store.error).toBeNull();
  });

  it('keeps the login URL on screen when the browser cannot open', async () => {
    const { store } = harness((method) => {
      if (method === RPC.cloudLoginBegin) return { browserUrl: 'https://cloud.example/desktop/login?x=2' };
      if (method === RPC.cloudLoginOpen) {
        throw new ControlCallError('internal', 'cannot open', { reason: 'failed' });
      }
      if (method === RPC.cloudLoginComplete) {
        return signedOut({ state: ACCOUNT_STATE.signedIn, accountId: 'user-2' });
      }
      throw new Error(`unexpected ${method}`);
    });
    await store.startLogin();
    // Manual opening still completes the loop; nothing secret is shown.
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedIn);
  });

  it('surfaces login failures without leaking tokens', async () => {
    const failure = new ControlCallError('auth_failed', 'cloud is unreachable', { reason: 'failed' });
    const { store } = harness((method) => {
      if (method === RPC.cloudLoginBegin) throw failure;
      if (method === RPC.cloudLoginCancel) return signedOut();
      if (method === RPC.cloudStatus) return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    await store.startLogin();
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedOut);
    expect(store.error).toContain('unreachable');
    expect(JSON.stringify(store.$state)).not.toContain('token');
  });

  it('cancels and logs out through the control plane', async () => {
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudLoginCancel) {
        return signedOut({ state: ACCOUNT_STATE.signedOut, error: 'login cancelled' });
      }
      if (method === RPC.cloudLogout) return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    store.status = signedOut({ state: 'signing-in', loginOperationId: '0123456789abcdef0123456789abcdef' });
    await store.cancelLogin();
    await store.logout();
    expect(calls.map((call) => call.method)).toEqual([RPC.cloudLoginCancel, RPC.cloudLogout]);
    expect(calls[0]?.params).toEqual({ operationId: '0123456789abcdef0123456789abcdef' });
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedOut);
  });

  it('restores the session explicitly after restart', async () => {
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudRestore) {
        return signedOut({ state: ACCOUNT_STATE.signedIn, accountId: 'user-3' });
      }
      throw new Error(`unexpected ${method}`);
    });
    await store.restore();
    expect(store.status?.accountId).toBe('user-3');
    expect(calls.map((call) => call.method)).toEqual([RPC.cloudRestore]);
    await flushPromises();
  });

  it('never lets an earlier status response undo logout', async () => {
    let release!: (value: AccountStatus) => void;
    const delayed = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store } = harness((method) => {
      if (method === RPC.cloudStatus) return delayed;
      if (method === RPC.cloudLogout) return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    store.status = signedOut({ state: ACCOUNT_STATE.signedIn, accountId: 'alice' });
    const old = store.refresh();
    await store.logout();
    release(signedOut({ state: ACCOUNT_STATE.signedIn, accountId: 'alice' }));
    await old;
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedOut);
    expect(store.loading).toBe(false);
    expect(store.working).toBe(false);
  });

  it('holds the loading fence through restore and rejects a reply after logout', async () => {
    let release!: (value: AccountStatus) => void;
    const delayed = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store } = harness((method) => {
      if (method === RPC.cloudStatus) return signedOut();
      if (method === RPC.cloudRestore) return delayed;
      if (method === RPC.cloudLogout) return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    const old = store.ensureRestored();
    await flushPromises();
    expect(store.loading).toBe(true);
    await store.logout();
    release(signedOut({ state: ACCOUNT_STATE.signedIn, accountId: 'alice' }));
    await old;
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedOut);
    expect(store.loading).toBe(false);
  });

  it('does not open or complete a login cancelled before begin replies', async () => {
    let release!: (value: { browserUrl: string }) => void;
    const delayed = new Promise<{ browserUrl: string }>((resolve) => { release = resolve; });
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudLoginBegin) return delayed;
      if (method === RPC.cloudLoginCancel) return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    const old = store.startLogin();
    await flushPromises();
    const cancelled = store.cancelLogin();
    expect(calls.map((entry) => entry.method)).toEqual([RPC.cloudLoginReserve, RPC.cloudLoginBegin, RPC.cloudLoginCancel]);
    release({ browserUrl: 'https://cloud.example/desktop/login' });
    await Promise.all([old, cancelled]);
    expect(calls.at(-1)?.params).toEqual({ operationId: '0123456789abcdef0123456789abcdef' });
    expect(store.loginUrl).toBeNull();
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedOut);
    expect(store.working).toBe(false);
  });

  it('does not wait for a superseded restore before a later refresh', async () => {
    let release!: (value: AccountStatus) => void;
    let requests = 0;
    const delayed = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store } = harness((method) => {
      if (method === RPC.cloudStatus) {
        requests += 1;
        return requests === 1 ? delayed : signedOut();
      }
      if (method === RPC.cloudLogout) return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    const old = store.ensureRestored();
    await store.logout();
    await store.refresh();
    expect(requests).toBe(2);
    release(signedOut({ state: ACCOUNT_STATE.signedIn, accountId: 'alice' }));
    await old;
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedOut);
  });

  it('waits for restore before login and keeps the restored account', async () => {
    let release!: (value: AccountStatus) => void;
    const pending = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudStatus) return signedOut();
      if (method === RPC.cloudRestore) return pending;
      throw new Error(`unexpected ${method}`);
    });
    const restored = store.ensureRestored();
    await flushPromises();
    const login = store.startLogin();
    expect(calls.map((entry) => entry.method)).toEqual([RPC.cloudStatus, RPC.cloudRestore]);
    release(signedOut({ state: ACCOUNT_STATE.signedIn, accountId: 'alice' }));
    await Promise.all([restored, login]);
    expect(store.status?.accountId).toBe('alice');
    expect(store.working).toBe(false);
    expect(store.loading).toBe(false);
  });

  it('waits for an explicit restore before refreshing status', async () => {
    let release!: (value: AccountStatus) => void;
    const pending = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudRestore) return pending;
      throw new Error(`unexpected ${method}`);
    });
    const restored = store.restore();
    const refreshed = store.refresh();
    expect(calls.map((entry) => entry.method)).toEqual([RPC.cloudRestore]);
    release(signedOut({ state: ACCOUNT_STATE.signedIn, accountId: 'alice' }));
    await Promise.all([restored, refreshed]);
    expect(store.status?.accountId).toBe('alice');
  });

  it('cancels a login waiting on restore without discarding its result', async () => {
    let release!: (value: AccountStatus) => void;
    const pending = new Promise<AccountStatus>((resolve) => { release = resolve; });
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudRestore) return pending;
      throw new Error(`unexpected ${method}`);
    });
    const restored = store.restore();
    const login = store.startLogin();
    const cancelled = store.cancelLogin();
    release(signedOut({ state: ACCOUNT_STATE.signedIn, accountId: 'alice' }));
    await Promise.all([restored, login, cancelled]);
    expect(calls.map((entry) => entry.method)).toEqual([RPC.cloudRestore]);
    expect(store.status?.accountId).toBe('alice');
    expect(store.working).toBe(false);
  });

  it('cancels the correlated attempt even if begin subsequently fails', async () => {
    let reject!: (error: Error) => void;
    const pending = new Promise<{ browserUrl: string }>((_, fail) => { reject = fail; });
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudLoginBegin) return pending;
      if (method === RPC.cloudLoginCancel) return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    const login = store.startLogin();
    await flushPromises();
    const cancelled = store.cancelLogin();
    reject(new Error('begin failed'));
    await Promise.all([login, cancelled]);
    expect(calls.map((entry) => entry.method)).toEqual([RPC.cloudLoginReserve, RPC.cloudLoginBegin, RPC.cloudLoginCancel]);
    expect(store.status?.state).toBe(ACCOUNT_STATE.signedOut);
    expect(store.working).toBe(false);
  });

  it('invalidates a begin whose response was lost before checking status', async () => {
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudLoginBegin) throw new Error('timed out');
      if (method === RPC.cloudLoginCancel || method === RPC.cloudStatus) return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    await store.startLogin();
    expect(calls.map((entry) => entry.method)).toEqual([
      RPC.cloudLoginReserve, RPC.cloudLoginBegin, RPC.cloudLoginCancel, RPC.cloudStatus,
    ]);
    expect(calls[2]?.params).toEqual({ operationId: '0123456789abcdef0123456789abcdef' });
    expect(store.error).toContain('timed out');
    expect(store.loginActive).toBe(false);
  });

  it('cancels the original operation after a complete deadline without replaying it', async () => {
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudLoginBegin) return { browserUrl: 'https://cloud.example/login' };
      if (method === RPC.cloudLoginOpen) return { ok: true };
      if (method === RPC.cloudLoginComplete) throw new ControlCallError('deadline', 'operation may still complete');
      if (method === RPC.cloudLoginCancel || method === RPC.cloudStatus) return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    await store.startLogin();
    expect(calls.map((entry) => entry.method)).toEqual([
      RPC.cloudLoginReserve, RPC.cloudLoginBegin, RPC.cloudLoginOpen, RPC.cloudLoginComplete,
      RPC.cloudLoginCancel, RPC.cloudStatus,
    ]);
    expect(calls[4]?.params).toEqual({ operationId: '0123456789abcdef0123456789abcdef' });
    expect(store.loginActive).toBe(false);
  });

  it('ignores a late reservation after cancellation and a newer login', async () => {
    let release!: (value: { operationId: string }) => void;
    const reservation = new Promise<{ operationId: string }>((resolve) => { release = resolve; });
    let finish!: (value: AccountStatus) => void;
    const completion = new Promise<AccountStatus>((resolve) => { finish = resolve; });
    let reservations = 0;
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudLoginBegin) return { browserUrl: 'https://cloud.example/login' };
      if (method === RPC.cloudLoginOpen) return { ok: true };
      if (method === RPC.cloudLoginComplete) return completion;
      if (method === RPC.cloudLoginCancel) return signedOut();
      throw new Error(`unexpected ${method}`);
    }, () => ++reservations === 1 ? reservation : { operationId: '0123456789abcdef0123456789abcdef' });
    const login = store.startLogin();
    await store.cancelLogin();
    const newer = store.startLogin();
    await flushPromises();
    release({ operationId: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' });
    await login;
    expect(calls.filter((entry) => entry.method === RPC.cloudLoginBegin).map((entry) => entry.params))
      .toEqual([{ operationId: '0123456789abcdef0123456789abcdef' }]);
    expect(calls.filter((entry) => entry.method === RPC.cloudLoginCancel).map((entry) => entry.params))
      .toEqual([{ operationId: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' }]);
    expect(store.loginActive).toBe(true);
    await store.cancelLogin();
    finish(signedOut());
    await newer;
  });

  it('does not begin when the reservation response times out', async () => {
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudStatus) return signedOut();
      throw new Error(`unexpected ${method}`);
    }, () => { throw new Error('reservation timed out'); });
    await store.startLogin();
    expect(calls.map((entry) => entry.method)).toEqual([RPC.cloudLoginReserve, RPC.cloudStatus]);
    expect(store.error).toContain('timed out');
  });

  it('does not send an unscoped begin for a malformed reservation', async () => {
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudStatus) return signedOut();
      throw new Error(`unexpected ${method}`);
    }, () => ({ operationId: null }));
    await store.startLogin();
    expect(calls.map((entry) => entry.method)).toEqual([RPC.cloudLoginReserve, RPC.cloudStatus]);
    expect(store.error).toContain('Invalid login reservation');
  });

  it('keeps a failed correlated cancellation available to retry', async () => {
    let release!: (value: { browserUrl: string }) => void;
    const pending = new Promise<{ browserUrl: string }>((resolve) => { release = resolve; });
    let cancels = 0;
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudLoginBegin) return pending;
      if (method === RPC.cloudLoginCancel) {
        if (++cancels === 1) throw new Error('cancel unavailable');
        return signedOut();
      }
      if (method === RPC.cloudStatus) return signedOut();
      throw new Error(`unexpected ${method}`);
    });
    const login = store.startLogin();
    await flushPromises();
    await store.cancelLogin();
    expect(store.loginActive).toBe(true);
    await store.startLogin();
    expect(calls.filter((entry) => entry.method === RPC.cloudLoginReserve)).toHaveLength(1);
    await store.cancelLogin();
    expect(store.loginActive).toBe(false);
    expect(calls.filter((entry) => entry.method === RPC.cloudLoginCancel).map((entry) => entry.params))
      .toEqual([{ operationId: '0123456789abcdef0123456789abcdef' }, { operationId: '0123456789abcdef0123456789abcdef' }]);
    release({ browserUrl: 'https://cloud.example/login' });
    await login;
  });


  it('reconciles native state when cancellation fails', async () => {
    const { store, calls } = harness((method) => {
      if (method === RPC.cloudLoginCancel) throw new Error('cancel unavailable');
      if (method === RPC.cloudStatus) return signedOut({ state: 'signing-in' });
      throw new Error(`unexpected ${method}`);
    });
    store.status = signedOut({ state: 'signing-in', loginOperationId: '0123456789abcdef0123456789abcdef' });
    await store.cancelLogin();
    expect(calls.map((entry) => entry.method)).toEqual([RPC.cloudLoginCancel, RPC.cloudStatus]);
    expect(store.status?.state).toBe('signing-in');
    expect(store.error).toContain('cancel unavailable');
    expect(store.working).toBe(false);
  });

});
