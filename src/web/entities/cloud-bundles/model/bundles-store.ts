import { defineStore } from 'pinia';
import { computed, ref, watch } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useCloudFailure } from '@/shared/api/cloud-failure.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import type { JobSnapshot } from '@/entities/plugin-package/api/package-types.ts';
import { packagePreflight, uploadAsset, type AssetRecord, type PackagePreflight } from '../api/bundles-api.ts';
import { importLibraryTemplate } from '../lib/library-import.ts';
import { importLibraryProfile } from '../lib/library-profile.ts';
import { validateOverlay, type OverlayDocument } from '../../overlay/lib/cloud-overlay-schema.ts';
import {
  createJournal,
  nextPending,
  parseJournal,
  recoverInterrupted,
  serializeJournal,
  setEntry,
  type ApplyJournal,
  type EntryStatus,
} from './apply-journal.ts';

export const MAX_IMAGE_BYTES = 512 * 1024;
export const JOURNAL_KEY = 'cloud.bundleApply';

type Outcome = { readonly status: EntryStatus; readonly message?: string };

/**
 * Cloud assets, package preflight and the client-side apply journal. The Cloud never
 * applies a package: each item goes through the host's own install or import flow,
 * and every step is written to the journal before the next one starts.
 */
export const useBundlesStore = defineStore('cloud-bundles', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();
  const packages = usePackagesStore();
  const account = useAccountStore();

  const asset = ref<AssetRecord | null>(null);
  const preflight = ref<PackagePreflight | null>(null);
  const packageId = ref<string | null>(null);
  const working = ref(false);
  const error = ref<string | null>(null);
  const { detail: errorDetail, capture } = useCloudFailure();
  const journal = ref<ApplyJournal | null>(null);
  const applying = ref(false);
  const applyError = ref<string | null>(null);
  let generation = 0;
  let cancelled = false;

  const available = computed(() => capabilities.supports('bundles.packages.preflight'));

  /** Uploads an image. Images over 512 KiB are refused before any request. */
  async function upload(file: File): Promise<boolean> {
    if (file.size === 0 || file.size > MAX_IMAGE_BYTES) {
      error.value = 'size';
      return false;
    }
    working.value = true;
    error.value = null;
    try {
      asset.value = await uploadAsset(client, new Uint8Array(await file.arrayBuffer()));
      return true;
    } catch (failure) {
      error.value = capture(failure);
      return false;
    } finally {
      working.value = false;
    }
  }

  /** Loads the preflight for a package id. Older replies never replace a newer one. */
  async function checkPackage(id: string): Promise<boolean> {
    const ticket = ++generation;
    packageId.value = id;
    working.value = true;
    error.value = null;
    preflight.value = null;
    try {
      const report = await packagePreflight(client, id);
      if (ticket === generation) preflight.value = report;
      return true;
    } catch (failure) {
      if (ticket === generation) error.value = capture(failure);
      return false;
    } finally {
      if (ticket === generation) working.value = false;
    }
  }

  async function persist(next: ApplyJournal): Promise<void> {
    journal.value = next;
    await client.call('app.state.set', { key: JOURNAL_KEY, value: serializeJournal(next) });
  }

  /** Reads the persisted journal. An interrupted step is recovered, never retried blindly. */
  async function loadJournal(): Promise<void> {
    const state = await client.call<{ state: Record<string, string> }>('app.state.get', { keys: [JOURNAL_KEY] });
    const text = state.state[JOURNAL_KEY];
    if (!text) return;
    const parsed = parseJournal(text);
    if (!parsed) return;
    await persist(recoverInterrupted(parsed));
  }

  /** Forgets a finished journal. */
  async function dismissJournal(): Promise<void> {
    if (applying.value) return;
    journal.value = null;
    await client.call('app.state.set', { key: JOURNAL_KEY, value: '' });
  }

  function until(predicate: () => boolean): Promise<void> {
    return new Promise((resolve) => {
      if (cancelled || predicate()) {
        resolve();
        return;
      }
      const stop = watch(
        () => cancelled || predicate(),
        (done) => {
          if (done) {
            stop();
            resolve();
          }
        },
      );
    });
  }

  function terminal(job: JobSnapshot | undefined): job is JobSnapshot {
    return job !== undefined && (job.status === 'succeeded' || job.status === 'failed' || job.status === 'cancelled');
  }

  /** Plugin steps reuse the install review. The step ends with the job the user confirmed. */
  async function runPlugin(id: string, version: string | null): Promise<Outcome> {
    const before = new Set(packages.jobs.map((job) => job.id));
    await packages.reviewSpec({ kind: 'registry', id, ...(version ? { version } : {}) });
    await until(() => !packages.review.open);
    if (cancelled) return { status: 'skipped', message: 'cancelled before this step' };
    const created = packages.jobs.find((job) => job.pluginId === id && !before.has(job.id));
    if (!created) return { status: 'skipped', message: 'the install was not confirmed' };
    await until(() => terminal(packages.jobs.find((job) => job.id === created.id)));
    const final = packages.jobs.find((job) => job.id === created.id);
    if (final?.status === 'succeeded') return { status: 'done' };
    if (final?.status === 'cancelled') return { status: 'skipped', message: 'the install was cancelled' };
    return { status: 'failed', message: final?.error?.message ?? 'the install failed' };
  }

  async function runTemplate(id: string, revision: number | null): Promise<Outcome> {
    const origin = account.status?.origin ?? null;
    const accountId = account.status?.accountId ?? null;
    if (!origin || !accountId) return { status: 'failed', message: 'sign in to Cloud to import this template' };
    if (revision === null) return { status: 'failed', message: 'the template has no revision' };
    try {
      await importLibraryTemplate(client, origin, accountId, id, revision);
      return { status: 'done' };
    } catch (failure) {
      return { status: 'failed', message: errorMessage(failure) };
    }
  }

  async function runProfile(id: string, revision: number | null): Promise<Outcome> {
    const origin = account.status?.origin ?? null;
    const accountId = account.status?.accountId ?? null;
    if (!origin || !accountId) return { status: 'failed', message: 'sign in to Cloud to import this profile' };
    if (revision === null) return { status: 'failed', message: 'the profile has no revision' };
    try {
      await importLibraryProfile(client, origin, accountId, id, revision);
      return { status: 'done' };
    } catch (failure) {
      return { status: 'failed', message: errorMessage(failure) };
    }
  }

  /**
   * An overlay is applied by reading the pinned revision and checking it with the
   * Cloud's rules. It then stays available in the overlay editor, which loads it by id
   * and copies its OBS URL.
   */
  async function runOverlay(id: string, revision: number | null): Promise<Outcome> {
    if (revision === null) return { status: 'failed', message: 'the overlay has no revision' };
    try {
      const record = await client.call<{ revision: number; document: unknown }>('bundles.overlays.read', { id });
      if (record.revision !== revision) return { status: 'failed', message: 'the overlay changed since it was packaged' };
      if (validateOverlay(record.document as OverlayDocument).length > 0) {
        return { status: 'failed', message: 'this overlay cannot be read by this version of the app' };
      }
      return { status: 'done' };
    } catch (failure) {
      return { status: 'failed', message: errorMessage(failure) };
    }
  }

  async function runEntry(index: number): Promise<Outcome> {
    const entry = journal.value?.entries[index];
    if (!entry) return { status: 'failed', message: 'the journal entry is missing' };
    if (entry.kind === 'plugin') return runPlugin(entry.id, entry.version);
    if (entry.kind === 'template') return runTemplate(entry.id, entry.revision);
    if (entry.kind === 'profile') return runProfile(entry.id, entry.revision);
    return runOverlay(entry.id, entry.revision);
  }

  /**
   * Applies a checked package. Refuses unless the last preflight says it can be
   * applied. Each step is persisted before it starts and after it ends.
   */
  async function applyPackage(): Promise<boolean> {
    const report = preflight.value;
    const id = packageId.value;
    if (!report || !id || !report.canApply || applying.value) return false;
    applyError.value = null;
    cancelled = false;
    applying.value = true;
    try {
      await persist(createJournal(id, report.items, Date.now()));
      let index = nextPending(journal.value as ApplyJournal);
      while (index >= 0) {
        await persist(setEntry(journal.value as ApplyJournal, index, 'running'));
        const outcome = await runEntry(index);
        await persist(setEntry(journal.value as ApplyJournal, index, outcome.status, outcome.message ?? null));
        if (cancelled) break;
        index = nextPending(journal.value as ApplyJournal);
      }
      if (cancelled && journal.value) {
        let next = journal.value;
        for (let position = 0; position < next.entries.length; position += 1) {
          if (next.entries[position]?.status === 'pending') next = setEntry(next, position, 'skipped', 'cancelled before this step');
        }
        await persist(next);
      }
      return true;
    } catch (failure) {
      applyError.value = errorMessage(failure);
      return false;
    } finally {
      applying.value = false;
    }
  }

  /** Stops after the current step. Remaining steps are recorded as skipped. */
  function cancelApply(): void {
    cancelled = true;
  }

  return {
    asset,
    preflight,
    packageId,
    working,
    error,
    errorDetail,
    available,
    journal,
    applying,
    applyError,
    upload,
    checkPackage,
    loadJournal,
    applyPackage,
    cancelApply,
    dismissJournal,
  };
});
