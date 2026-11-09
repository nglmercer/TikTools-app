import { defineStore } from 'pinia';
import { ref } from 'vue';
import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import type { AccountStatus, CloudLoginUrlResult } from '../api/account-types.ts';

export const useAccountStore = defineStore('cloud-account', () => {
  const client = useControlClient();
  const status = ref<AccountStatus | null>(null);
  const loading = ref(false);
  const working = ref(false);
  const loginUrl = ref<string | null>(null);
  const error = ref<string | null>(null);
  const loginActive = ref(false);
  // A late status/login/restore reply must never replace a newer account
  // transition, even if the same account is signed in again.
  let generation = 0;
  let reading: { ticket: number; kind: 'refresh' | 'restore'; promise: Promise<void> } | null = null;
  let loginBegin: Promise<CloudLoginUrlResult> | null = null;
  let cancelling: Promise<void> | null = null;
  let cancelRequested = false;
  let restored = false;
  const current = (ticket: number) => ticket === generation;

  function read(kind: 'refresh' | 'restore', run: (ticket: number) => Promise<void>): Promise<void> {
    const ticket = ++generation;
    loading.value = true;
    const promise = (async () => {
      try { await run(ticket); }
      finally {
        if (current(ticket)) loading.value = false;
        if (reading?.ticket === ticket) reading = null;
      }
    })();
    reading = { ticket, kind, promise };
    return promise;
  }

  function refresh(): Promise<void> {
    if (reading && current(reading.ticket)) return reading.promise;
    if (working.value) return Promise.resolve();
    return read('refresh', async (ticket) => {
      try {
        const next = await client.call<AccountStatus>('cloud.status', {});
        if (!current(ticket)) return;
        status.value = next;
        if (next.state !== 'signing-in') loginUrl.value = null;
        error.value = next.error ?? null;
      } catch (failure) {
        if (current(ticket)) error.value = errorMessage(failure);
      }
    });
  }

  function restore(): Promise<void> {
    if (reading && current(reading.ticket)) {
      const previous = reading;
      if (previous.kind === 'restore') return previous.promise;
      return previous.promise.then(() => current(previous.ticket) ? restore() : undefined);
    }
    if (working.value) return Promise.resolve();
    return read('restore', async (ticket) => {
      try {
        const next = await client.call<AccountStatus>('cloud.restore', {});
        if (!current(ticket)) return;
        status.value = next;
        error.value = next.error ?? null;
      } catch (failure) {
        if (current(ticket)) error.value = errorMessage(failure);
      } finally {
        if (current(ticket)) restored = true;
      }
    });
  }

  /** Concurrent callers share one status and saved-session restore attempt. */
  function ensureRestored(): Promise<void> {
    if (reading && current(reading.ticket)) {
      const previous = reading;
      if (previous.kind === 'restore') return previous.promise;
      return previous.promise.then(() => current(previous.ticket) ? ensureRestored() : undefined);
    }
    if (working.value) return Promise.resolve();
    if (restored) return refresh();
    return read('restore', async (ticket) => {
      try {
        const initial = await client.call<AccountStatus>('cloud.status', {});
        if (!current(ticket)) return;
        status.value = initial;
        error.value = initial.error ?? null;
        if (initial.state !== 'signed-out') return;
        const next = await client.call<AccountStatus>('cloud.restore', {});
        if (!current(ticket)) return;
        status.value = next;
        error.value = next.error ?? null;
      } catch (failure) {
        if (!current(ticket)) return;
        const reason = failure instanceof ControlCallError
          ? (failure.data as { reason?: string } | undefined)?.reason
          : undefined;
        if (reason !== 'no-saved-session' && reason !== 'unconfigured') error.value = errorMessage(failure);
      } finally {
        if (current(ticket)) restored = true;
      }
    });
  }

  async function startLogin(): Promise<void> {
    if (working.value) return;
    // Reserve the action without invalidating an in-flight restore. The native
    // restore must settle before begin is sent, or it can finish behind us.
    working.value = true;
    loginActive.value = true;
    cancelRequested = false;
    error.value = null;
    let ticket = generation;
    let began = false;
    try {
      if (reading && current(reading.ticket)) await reading.promise;
      if (!current(ticket) || cancelRequested) return;
      if (status.value?.state === 'signed-in') return;
      ticket = ++generation;
      const begin = client.call<CloudLoginUrlResult>('cloud.login.begin', {});
      loginBegin = begin;
      began = true;
      const begun = await begin;
      if (!current(ticket)) return;
      loginUrl.value = begun.browserUrl;
      try {
        await client.call('cloud.login.open', {});
      } catch {
        // The URL stays on screen for manual opening; not fatal.
      }
      if (!current(ticket)) return;
      const next = await client.call<AccountStatus>('cloud.login.complete', {});
      if (!current(ticket)) return;
      status.value = next;
      error.value = next.error ?? null;
      loginUrl.value = null;
    } catch (failure) {
      if (!current(ticket)) return;
      error.value = errorMessage(failure);
      try {
        const next = await client.call<AccountStatus>('cloud.status', {});
        if (current(ticket)) status.value = next;
      } catch {
        // Keep the original failure.
      }
    } finally {
      if (current(ticket)) {
        if (began) restored = true;
        loginBegin = null;
        if (!cancelRequested) { loginActive.value = false; working.value = false; }
      }
    }
  }

  function cancelLogin(): Promise<void> {
    if (cancelling) return cancelling;
    const begin = loginBegin;
    const waitingForRead = !begin && loginActive.value && reading && current(reading.ticket) ? reading : null;
    cancelRequested = true;
    const ticket = waitingForRead ? generation : ++generation;
    let failed = false;
    working.value = true;
    if (!waitingForRead) loading.value = false;
    cancelling = (async () => {
      try {
        if (waitingForRead) {
          await waitingForRead.promise;
          return;
        }
        // The native cancel API cannot cancel a begin that has not yet
        // installed its pending attempt. Wait until it exists first.
        if (begin) {
          try { await begin; }
          catch {
            const next = await client.call<AccountStatus>('cloud.status', {});
            if (current(ticket)) { status.value = next; error.value = next.error ?? null; }
            return;
          }
        } else if (loginActive.value) {
          // Login was waiting for a read; no native login has started.
          return;
        }
        const next = await client.call<AccountStatus>('cloud.login.cancel', {});
        if (!current(ticket)) return;
        status.value = next;
        error.value = next.error ?? null;
      } catch (failure) {
        if (!current(ticket)) return;
        failed = true;
        error.value = errorMessage(failure);
        // A failed cancel may leave a native attempt pending. Reconcile its
        // state so that the user can retry cancellation instead of hiding it.
        try {
          const next = await client.call<AccountStatus>('cloud.status', {});
          if (current(ticket)) status.value = next;
        } catch {
          // Preserve the cancellation error.
        }
      } finally {
        if (current(ticket)) {
          if (!waitingForRead) restored = true;
          loginBegin = null;
          if (!failed) loginUrl.value = null;
          loginActive.value = false;
          cancelRequested = false;
          working.value = false;
        }
        cancelling = null;
      }
    })();
    return cancelling;
  }

  async function logout(): Promise<void> {
    if (working.value) return;
    const ticket = ++generation;
    working.value = true;
    loading.value = false;
    try {
      const next = await client.call<AccountStatus>('cloud.logout', {});
      if (!current(ticket)) return;
      status.value = next;
      error.value = status.value.error ?? null;
      loginUrl.value = null;
    } catch (failure) {
      if (current(ticket)) error.value = errorMessage(failure);
    } finally {
      if (current(ticket)) { restored = true; working.value = false; }
    }
  }

  return { status, loading, working, loginActive, loginUrl, error, refresh, restore, ensureRestored, startLogin, cancelLogin, logout };
});
