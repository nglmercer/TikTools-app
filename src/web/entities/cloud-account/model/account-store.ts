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
  // A late status/login/restore reply must never replace a newer account
  // transition, even if the same account is signed in again.
  let generation = 0;
  let restoring: { ticket: number; promise: Promise<void> } | null = null;
  let restored = false;
  const current = (ticket: number) => ticket === generation;

  async function refresh(): Promise<void> {
    if (restoring && current(restoring.ticket)) return restoring.promise;
    if (working.value) return;
    const ticket = ++generation;
    loading.value = true;
    try {
      const next = await client.call<AccountStatus>('cloud.status', {});
      if (!current(ticket)) return;
      status.value = next;
      if (status.value.state !== 'signing-in') loginUrl.value = null;
      error.value = status.value.error ?? null;
    } catch (failure) {
      if (current(ticket)) error.value = errorMessage(failure);
    } finally {
      if (current(ticket)) loading.value = false;
    }
  }

  async function restore(): Promise<void> {
    if (working.value) return;
    const ticket = ++generation;
    loading.value = true;
    try {
      const next = await client.call<AccountStatus>('cloud.restore', {});
      if (!current(ticket)) return;
      status.value = next;
      error.value = status.value.error ?? null;
    } catch (failure) {
      if (current(ticket)) error.value = errorMessage(failure);
    } finally {
      if (current(ticket)) loading.value = false;
    }
  }

  /**
   * Load the status and, once per app session, reconnect a session saved in
   * the OS keychain. "Nothing saved" is a normal signed-out start and stays
   * silent; real failures (expired session, keychain unavailable) surface.
   * Concurrent callers share one attempt.
   */
  function ensureRestored(): Promise<void> {
    if (restoring && current(restoring.ticket)) return restoring.promise;
    if (working.value) return Promise.resolve();
    // After the one restore attempt, later mounts just reload the status.
    if (restored) return refresh();
    const ticket = ++generation;
    loading.value = true;
    const promise = (async () => {
      try {
        const initial = await client.call<AccountStatus>('cloud.status', {});
        if (!current(ticket)) return;
        status.value = initial;
        error.value = initial.error ?? null;
        if (initial.state !== 'signed-out') return;
        const next = await client.call<AccountStatus>('cloud.restore', {});
        if (!current(ticket)) return;
        status.value = next;
        error.value = status.value.error ?? null;
      } catch (failure) {
        if (!current(ticket)) return;
        const reason = failure instanceof ControlCallError
          ? (failure.data as { reason?: string } | undefined)?.reason
          : undefined;
        if (reason !== 'no-saved-session' && reason !== 'unconfigured') error.value = errorMessage(failure);
      } finally {
        if (current(ticket)) { restored = true; loading.value = false; }
        if (restoring?.ticket === ticket) restoring = null;
      }
    })();
    restoring = { ticket, promise };
    return promise;
  }

  async function startLogin(): Promise<void> {
    if (working.value) return;
    const ticket = ++generation;
    working.value = true;
    loading.value = false;
    error.value = null;
    try {
      const begun = await client.call<CloudLoginUrlResult>('cloud.login.begin', {});
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
      error.value = status.value.error ?? null;
      loginUrl.value = null;
    } catch (failure) {
      if (!current(ticket)) return;
      error.value = errorMessage(failure);
      // Sync state without letting a clean status wipe the failure text.
      try {
        const next = await client.call<AccountStatus>('cloud.status', {});
        if (current(ticket)) status.value = next;
      } catch {
        // Keep the original failure.
      }
    } finally {
      if (current(ticket)) { restored = true; working.value = false; }
    }
  }

  async function cancelLogin(): Promise<void> {
    const ticket = ++generation;
    working.value = true;
    loading.value = false;
    try {
      const next = await client.call<AccountStatus>('cloud.login.cancel', {});
      if (!current(ticket)) return;
      status.value = next;
      error.value = status.value.error ?? null;
    } catch (failure) {
      if (current(ticket)) error.value = errorMessage(failure);
    } finally {
      if (current(ticket)) { restored = true; loginUrl.value = null; working.value = false; }
    }
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

  return { status, loading, working, loginUrl, error, refresh, restore, ensureRestored, startLogin, cancelLogin, logout };
});
