import { defineStore } from 'pinia';
import { ref } from 'vue';
import { ControlCallError, errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import type {
  AccountPlan,
  AccountStatus,
  BackupManifest,
  BackupSummary,
  CloudLoginUrlResult,
  LoginCodeStatus,
  StoredBackup,
} from '../api/account-types.ts';

export const useAccountStore = defineStore('cloud-account', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();
  const status = ref<AccountStatus | null>(null);
  const loading = ref(false);
  const working = ref(false);
  const loginUrl = ref<string | null>(null);
  const loginCode = ref<LoginCodeStatus | null>(null);
  const error = ref<string | null>(null);
  const plan = ref<AccountPlan | null>(null);
  const planError = ref<string | null>(null);
  const backupWorking = ref(false);
  const backupError = ref<string | null>(null);
  const backups = ref<StoredBackup[]>([]);
  const backupsLoading = ref(false);
  const loginActive = ref(false);
  // A late status/login/restore reply must never replace a newer account
  // transition, even if the same account is signed in again.
  let generation = 0;
  let reading: { ticket: number; kind: 'refresh' | 'restore'; promise: Promise<void> } | null = null;
  let loginReservation: Promise<{ operationId: string }> | null = null;
  let loginOperationId: string | null = null;
  let cancelling: Promise<void> | null = null;
  let cancelRequested = false;
  let codeSubmitter: (() => void) | null = null;
  let restored = false;
  const current = (ticket: number) => ticket === generation;

  function read(kind: 'refresh' | 'restore', run: (ticket: number) => Promise<void>): Promise<void> {
    // A host without Cloud features is not a failed sign-in: skip the call and keep the state empty.
    if (capabilities.supports('cloud.status') === false) return Promise.resolve();
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
        loginCode.value = next.loginCode ?? null;
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
    let restoreFailed = false;
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
        restoreFailed = true;
        const reason = failure instanceof ControlCallError
          ? (failure.data as { reason?: string } | undefined)?.reason
          : undefined;
        // A missing saved session is a normal signed-out start; the native
        // side records real restore failures in `cloud.status.error`.
        let detail = errorMessage(failure);
        try {
          const latest = await client.call<AccountStatus>('cloud.status', {});
          if (latest.error) detail = latest.error;
          else if (reason === 'unconfigured' || reason === 'no-saved-session') detail = '';
        } catch { /* keep the control error */ }
        if (current(ticket)) error.value = detail || null;
      } finally {
        // A failed restore must be retried instead of being latched as done.
        if (current(ticket) && !restoreFailed) restored = true;
      }
    });
  }

  async function startLogin(): Promise<void> {
    if (working.value || loginActive.value || loginOperationId || status.value?.state === 'signing-in') return;
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
      const reservation = client.call<{ operationId: string }>('cloud.login.reserve', {});
      loginReservation = reservation;
      const { operationId } = await reservation;
      if (!current(ticket) || cancelRequested) return;
      if (typeof operationId !== 'string' || !/^[0-9a-f]{32}$/.test(operationId)) {
        throw new Error('Invalid login reservation');
      }
      loginOperationId = operationId;
      const params = { operationId };
      const begin = client.call<CloudLoginUrlResult>('cloud.login.begin', params);
      began = true;
      const begun = await begin;
      if (!current(ticket)) return;
      loginUrl.value = begun.browserUrl;
      try {
        await client.call('cloud.login.open', params);
      } catch {
        // The URL stays on screen for manual opening; not fatal.
      }
      if (!current(ticket)) return;
      // The native callback waits up to 180 seconds; allow time for its
      // bounded network redemption and response before canceling uncertainty.
      const next = await client.call<AccountStatus>('cloud.login.complete', params, { timeoutMs: 300_000 });
      if (!current(ticket)) return;
      status.value = next;
      error.value = next.error ?? null;
      loginUrl.value = null;
      loginOperationId = null;
    } catch (failure) {
      if (!current(ticket)) return;
      error.value = errorMessage(failure);
      // A rejected or timed-out begin may still be running in native code.
      // Invalidate its reservation before allowing another login.
      if (loginOperationId) {
        try {
          await client.call<AccountStatus>('cloud.login.cancel', { operationId: loginOperationId });
          if (!current(ticket)) return;
          loginOperationId = null;
        } catch {
          // Keep the operation and Cancel button available for retry.
        }
      }
      try {
        const next = await client.call<AccountStatus>('cloud.status', {});
        if (current(ticket)) status.value = next;
      } catch {
        // Keep the original failure.
      }
    } finally {
      if (current(ticket)) {
        if (began) restored = true;
        loginReservation = null;
        if (!cancelRequested) { loginActive.value = loginOperationId !== null; working.value = false; }
      }
    }
  }

  async function startLoginCode(email: string): Promise<void> {
    if (working.value || loginActive.value || status.value?.state === 'signed-in') return;
    working.value = true;
    loginActive.value = true;
    error.value = null;
    const ticket = ++generation;
    let operationId: string | null = null;
    try {
      if (reading && current(reading.ticket)) await reading.promise;
      if (!current(ticket)) return;
      const reservation = await client.call<{ operationId: string }>('cloud.login.reserve', {});
      if (!current(ticket)) return;
      if (typeof reservation.operationId !== 'string' || !/^[0-9a-f]{32}$/.test(reservation.operationId)) {
        throw new Error('Invalid login reservation');
      }
      operationId = reservation.operationId;
      loginOperationId = operationId;
      const params = { operationId };
      const result = await client.call<LoginCodeStatus>('cloud.login.code.send', { ...params, email });
      if (!current(ticket)) return;
      loginCode.value = result;
      if (result.requiresOauth) {
        // No account: fall back to the browser/OAuth handoff.
        const begun = await client.call<CloudLoginUrlResult>('cloud.login.begin', params);
        if (!current(ticket)) return;
        loginUrl.value = begun.browserUrl;
        try { await client.call('cloud.login.open', params); } catch { /* manual open remains */ }
        return;
      }
      if (result.delivered === 0) {
        throw new Error('No connected devices: remote control needs at least one other device');
      }
      // Wait for the user to enter the code in the UI; submitLoginCode resolves it.
      await new Promise<void>((resolve) => { codeSubmitter = resolve; });
    } catch (failure) {
      if (!current(ticket)) return;
      error.value = errorMessage(failure);
      if (operationId) {
        try { await client.call<AccountStatus>('cloud.login.cancel', { operationId }); } catch { /* keep Cancel */ }
      }
      loginCode.value = null;
    } finally {
      if (current(ticket)) {
        codeSubmitter = null;
        if (loginActive.value) { working.value = false; }
      }
    }
  }

  async function resendLoginCode(): Promise<void> {
    if (!loginOperationId || !working.value) return;
    try {
      const result = await client.call<LoginCodeStatus>('cloud.login.code.resend', { operationId: loginOperationId });
      if (current(generation)) { loginCode.value = result; error.value = null; }
    } catch (failure) {
      if (current(generation)) error.value = errorMessage(failure);
    }
  }

  async function submitLoginCode(code: string): Promise<void> {
    if (!loginOperationId || !/^\d{6}$/.test(code)) {
      error.value = 'Enter the 6-digit code';
      return;
    }
    working.value = true;
    error.value = null;
    const ticket = generation;
    try {
      const next = await client.call<AccountStatus>('cloud.login.code.submit', { operationId: loginOperationId, code });
      if (!current(ticket)) return;
      status.value = next;
      loginCode.value = null;
      loginUrl.value = null;
      loginOperationId = null;
      loginActive.value = false;
      codeSubmitter?.();
      error.value = next.error ?? null;
    } catch (failure) {
      if (current(ticket)) error.value = errorMessage(failure);
    } finally {
      if (current(ticket)) working.value = false;
    }
  }

  function cancelLogin(): Promise<void> {
    if (cancelling) return cancelling;
    const reservation = loginReservation;
    const operationId = loginOperationId
      ?? (status.value?.state === 'signing-in' ? status.value.loginOperationId ?? null : null);
    const waitingForRead = !reservation && !operationId && loginActive.value && reading && current(reading.ticket) ? reading : null;
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
        const id = operationId;
        if (!id && reservation) {
          // No begin was sent. Release the UI immediately; a late reservation
          // is inert and the generation fence prevents it from starting one.
          // Discard it best-effort once its ID arrives to free native capacity.
          void reservation.then(({ operationId: lateId }) => {
            if (typeof lateId === 'string' && /^[0-9a-f]{32}$/.test(lateId)) {
              void client.call('cloud.login.cancel', { operationId: lateId }).catch(() => {});
            }
          }).catch(() => {});
          await Promise.resolve();
          return;
        }
        if (typeof id !== 'string' || !/^[0-9a-f]{32}$/.test(id)) {
          throw new Error('Cannot cancel without the observed login reservation');
        }
        if (id) loginOperationId = id;
        const next = await client.call<AccountStatus>('cloud.login.cancel', { operationId: id });
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
          loginReservation = null;
          if (!failed) loginOperationId = null;
          if (!failed) loginUrl.value = null;
          loginActive.value = failed && loginOperationId !== null;
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
      loginOperationId = null;
    } catch (failure) {
      if (current(ticket)) error.value = errorMessage(failure);
    } finally {
      if (current(ticket)) { restored = true; working.value = false; }
    }
  }

  async function createBackup(): Promise<BackupManifest | null> {
    if (capabilities.supports('cloud.backup.create') === false) return null;
    backupWorking.value = true;
    backupError.value = null;
    try {
      return await client.call<BackupManifest>('cloud.backup.create', {});
    } catch (failure) {
      backupError.value = errorMessage(failure);
      return null;
    } finally {
      backupWorking.value = false;
    }
  }

  async function previewBackup(manifest: BackupManifest): Promise<BackupSummary | null> {
    if (capabilities.supports('cloud.backup.preview') === false) return null;
    backupWorking.value = true;
    backupError.value = null;
    try {
      return await client.call<BackupSummary>('cloud.backup.preview', { manifest, mode: 'replace', dryRun: true });
    } catch (failure) {
      backupError.value = errorMessage(failure);
      return null;
    } finally {
      backupWorking.value = false;
    }
  }

  async function restoreBackup(manifest: BackupManifest): Promise<BackupSummary | null> {
    if (capabilities.supports('cloud.backup.restore') === false) return null;
    backupWorking.value = true;
    backupError.value = null;
    try {
      return await client.call<BackupSummary>('cloud.backup.restore', { manifest, mode: 'replace', dryRun: false });
    } catch (failure) {
      backupError.value = errorMessage(failure);
      return null;
    } finally {
      backupWorking.value = false;
    }
  }

  async function refreshBackups(): Promise<void> {
    if (capabilities.supports('cloud.backup.list') === false) return;
    backupsLoading.value = true;
    backupError.value = null;
    try {
      const result = await client.call<{ items: StoredBackup[] }>('cloud.backup.list', {});
      backups.value = result.items ?? [];
    } catch (failure) {
      backupError.value = errorMessage(failure);
    } finally {
      backupsLoading.value = false;
    }
  }

  /**
   * Stores a backup under `name`. With a `manifest` (a local file) that exact
   * content is uploaded; without one the cloud snapshots the account's data.
   */
  async function createNamedBackup(name: string, manifest?: BackupManifest): Promise<StoredBackup | null> {
    if (capabilities.supports('cloud.backup.createNamed') === false) return null;
    backupWorking.value = true;
    backupError.value = null;
    try {
      const item = await client.call<StoredBackup>('cloud.backup.createNamed', manifest ? { name, manifest } : { name });
      backups.value = [item, ...backups.value];
      void refreshPlan();
      return item;
    } catch (failure) {
      backupError.value = errorMessage(failure);
      return null;
    } finally {
      backupWorking.value = false;
    }
  }

  async function getStoredBackup(id: string): Promise<unknown | null> {
    if (capabilities.supports('cloud.backup.get') === false) return null;
    try {
      return await client.call('cloud.backup.get', { id });
    } catch (failure) {
      backupError.value = errorMessage(failure);
      return null;
    }
  }

  async function deleteStoredBackup(id: string): Promise<boolean> {
    if (capabilities.supports('cloud.backup.delete') === false) return false;
    backupWorking.value = true;
    backupError.value = null;
    try {
      await client.call('cloud.backup.delete', { id });
      backups.value = backups.value.filter((backup) => backup.id !== id);
      void refreshPlan();
      return true;
    } catch (failure) {
      backupError.value = errorMessage(failure);
      return false;
    } finally {
      backupWorking.value = false;
    }
  }

  async function restoreStoredBackup(id: string, dryRun: boolean): Promise<BackupSummary | null> {
    if (capabilities.supports('cloud.backup.restoreStored') === false) return null;
    backupWorking.value = true;
    backupError.value = null;
    try {
      return await client.call<BackupSummary>('cloud.backup.restoreStored', {
        id,
        mode: 'replace',
        dryRun,
      });
    } catch (failure) {
      backupError.value = errorMessage(failure);
      return null;
    } finally {
      backupWorking.value = false;
    }
  }

  /** Plan and storage usage of the signed-in account. A host without the method shows nothing. */
  async function refreshPlan(): Promise<void> {
    if (capabilities.supports('cloud.plan.get') === false) return;
    try {
      plan.value = await client.call<AccountPlan>('cloud.plan.get', {});
      planError.value = null;
    } catch (failure) {
      plan.value = null;
      planError.value = errorMessage(failure);
    }
  }

  return {
    status, loading, working, loginActive, loginUrl, loginCode, error, plan, planError,
    backupWorking, backupError, backups, backupsLoading,
    createBackup, previewBackup, restoreBackup,
    refreshBackups, createNamedBackup, getStoredBackup, deleteStoredBackup, restoreStoredBackup,
    refreshPlan,
    refresh, restore, ensureRestored, startLogin, startLoginCode, resendLoginCode,
    submitLoginCode, cancelLogin, logout,
  };
});
