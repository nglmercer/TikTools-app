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

  async function refresh(): Promise<void> {
    loading.value = true;
    try {
      status.value = await client.call<AccountStatus>('cloud.status', {});
      if (status.value.state !== 'signing-in') loginUrl.value = null;
      error.value = status.value.error ?? null;
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      loading.value = false;
    }
  }

  async function restore(): Promise<void> {
    loading.value = true;
    try {
      status.value = await client.call<AccountStatus>('cloud.restore', {});
      error.value = status.value.error ?? null;
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      loading.value = false;
    }
  }

  let restoring: Promise<void> | null = null;
  let restored = false;

  /**
   * Load the status and, once per app session, reconnect a session saved in
   * the OS keychain. "Nothing saved" is a normal signed-out start and stays
   * silent; real failures (expired session, keychain unavailable) surface.
   * Concurrent callers share one attempt.
   */
  function ensureRestored(): Promise<void> {
    // After the one restore attempt, later mounts just reload the status.
    if (restored) return refresh();
    restoring ??= (async () => {
      await refresh();
      if (status.value?.state !== 'signed-out') return;
      try {
        status.value = await client.call<AccountStatus>('cloud.restore', {});
        error.value = status.value.error ?? null;
      } catch (failure) {
        const reason = failure instanceof ControlCallError
          ? (failure.data as { reason?: string } | undefined)?.reason
          : undefined;
        if (reason !== 'no-saved-session' && reason !== 'unconfigured') error.value = errorMessage(failure);
      } finally {
        restored = true;
      }
    })();
    return restoring;
  }

  async function startLogin(): Promise<void> {
    if (working.value) return;
    working.value = true;
    error.value = null;
    try {
      const begun = await client.call<CloudLoginUrlResult>('cloud.login.begin', {});
      loginUrl.value = begun.browserUrl;
      try {
        await client.call('cloud.login.open', {});
      } catch {
        // The URL stays on screen for manual opening; not fatal.
      }
      status.value = await client.call<AccountStatus>('cloud.login.complete', {});
      error.value = status.value.error ?? null;
      loginUrl.value = null;
    } catch (failure) {
      error.value = errorMessage(failure);
      // Sync state without letting a clean status wipe the failure text.
      try {
        status.value = await client.call<AccountStatus>('cloud.status', {});
      } catch {
        // Keep the original failure.
      }
    } finally {
      working.value = false;
    }
  }

  async function cancelLogin(): Promise<void> {
    try {
      status.value = await client.call<AccountStatus>('cloud.login.cancel', {});
      error.value = status.value.error ?? null;
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      loginUrl.value = null;
      working.value = false;
    }
  }

  async function logout(): Promise<void> {
    if (working.value) return;
    working.value = true;
    try {
      status.value = await client.call<AccountStatus>('cloud.logout', {});
      error.value = status.value.error ?? null;
      loginUrl.value = null;
    } catch (failure) {
      error.value = errorMessage(failure);
    } finally {
      working.value = false;
    }
  }

  return { status, loading, working, loginUrl, error, refresh, restore, ensureRestored, startLogin, cancelLogin, logout };
});
