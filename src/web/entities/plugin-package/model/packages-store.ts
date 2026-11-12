import { defineStore } from 'pinia';
import { computed, ref, shallowRef } from 'vue';
import { useAutomationStore } from '@/entities/automation/model/automation-store.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import type {
  InstallError,
  InstallReceipt,
  JobSnapshot,
  PackagePreview,
  PackageSpecInput,
  PluginVersions,
  UpdatePolicy,
  UpdateReport,
} from '../api/package-types.ts';
import { installFailure, reviewFromError } from '../lib/install-error.ts';
import { isActiveJob } from '../lib/package-format.ts';
import { createPackageRegistryState } from './package-registry-state.ts';
import { createPackageUpdateReport } from './package-update-report.ts';

const MAX_JOBS = 50;

export type ReviewMode = 'install' | 'update';

export interface ReviewState {
  open: boolean;
  mode: ReviewMode;
  loading: boolean;
  submitting: boolean;
  spec: PackageSpecInput | null;
  preview: PackagePreview | null;
  error: InstallError | null;
}

function closedReview(): ReviewState {
  return {
    open: false,
    mode: 'install',
    loading: false,
    submitting: false,
    spec: null,
    preview: null,
    error: null,
  };
}

export const usePackagesStore = defineStore('plugin-packages', () => {
  const client = useControlClient();
  const automation = useAutomationStore();
  const registry = createPackageRegistryState(client);
  const updateReport = createPackageUpdateReport(client);
  const { updates, checkingUpdates, updatesError, updateCount, loadUpdates, checkUpdates } = updateReport;

  const jobs = shallowRef<JobSnapshot[]>([]);
  const receipts = ref<Record<string, InstallReceipt>>({});
  const versions = ref<Record<string, PluginVersions>>({});
  const versionsError = ref<Record<string, string>>({});
  const review = ref<ReviewState>(closedReview());
  const versionRequests = new Map<string, number>();
  const pendingUpdates = new Map<string, { pluginId: string; version?: string }>();
  let subscribed = false;
  let reviewRequest = 0;

  const activeJobs = computed(() => jobs.value.filter(isActiveJob));

  function upsertJob(job: JobSnapshot): void {
    const rest = jobs.value.filter((entry) => entry.id !== job.id);
    jobs.value = [job, ...rest].slice(0, MAX_JOBS);
    if (job.status === 'succeeded') {
      void automation.refresh();
      void loadReceipts();
      if (job.pluginId && versions.value[job.pluginId]) void loadVersions(job.pluginId);
      if (updates.value && job.pluginId) {
        updates.value = {
          ...updates.value,
          updates: updates.value.updates.filter((entry) => entry.id !== job.pluginId),
        };
      }
    }
    const pending = pendingUpdates.get(job.id);
    if (pending && job.status !== 'queued' && job.status !== 'running') {
      pendingUpdates.delete(job.id);
      const preview = job.error ? reviewFromError(job.error) : null;
      if (preview) {
        review.value = {
          ...closedReview(),
          open: true,
          mode: 'update',
          spec: { kind: 'registry', id: pending.pluginId, version: preview.version },
          preview,
        };
      }
    }
  }

  function subscribe(): void {
    if (subscribed) return;
    subscribed = true;
    client.onTopic<{ job: JobSnapshot }>('plugin.job', (data) => {
      if (data?.job) upsertJob(data.job);
    });
    client.onTopic<{ report: UpdateReport }>('plugin.updates', (data) => {
      // Notifications may be delayed or share a wall-clock timestamp. Treat
      // them as invalidations and read the host's current report instead.
      if (!data?.report) return;
      updateReport.invalidateFromNotification();
    });
    client.onTopic('plugin.updated', () => void automation.refresh());
  }

  async function initialize(): Promise<void> {
    subscribe();
    await Promise.all([loadReceipts(), loadJobs(), loadUpdates()]);
  }

  async function loadReceipts(): Promise<void> {
    try {
      const result = await client.call<{ receipts: InstallReceipt[] }>('plugins.packages.receipts', {});
      receipts.value = Object.fromEntries(result.receipts.map((receipt) => [receipt.id, receipt]));
    } catch {
      receipts.value = {};
    }
  }

  async function loadJobs(): Promise<void> {
    try {
      const result = await client.call<{ jobs: JobSnapshot[] }>('plugins.jobs.list', {});
      jobs.value = result.jobs.slice(0, MAX_JOBS);
    } catch {
      jobs.value = [];
    }
  }

  async function reviewSpec(spec: PackageSpecInput): Promise<void> {
    const request = ++reviewRequest;
    review.value = { ...closedReview(), open: true, loading: true, spec };
    try {
      const preview = await client.call<PackagePreview>('plugins.packages.inspect', { spec });
      if (request !== reviewRequest) return;
      review.value = { ...review.value, loading: false, preview };
    } catch (failure) {
      if (request !== reviewRequest) return;
      review.value = { ...review.value, loading: false, error: installFailure(failure) };
    }
  }

  async function confirmReview(policy?: UpdatePolicy): Promise<JobSnapshot | null> {
    const { spec, preview, mode } = review.value;
    if (!spec || !preview || review.value.submitting) return null;
    review.value = { ...review.value, submitting: true, error: null };
    try {
      const result =
        mode === 'update'
          ? await client.call<{ job: JobSnapshot }>('plugins.packages.update', {
              pluginId: preview.id,
              version: preview.version,
              consentToken: preview.consentToken,
            })
          : await client.call<{ job: JobSnapshot }>('plugins.packages.install', {
              spec,
              consentToken: preview.consentToken,
              ...(policy ? { policy } : {}),
            });
      upsertJob(result.job);
      review.value = closedReview();
      return result.job;
    } catch (failure) {
      review.value = { ...review.value, submitting: false, error: installFailure(failure) };
      return null;
    }
  }

  function closeReview(): void {
    reviewRequest += 1;
    review.value = closedReview();
  }

  async function updatePlugin(pluginId: string, version?: string): Promise<void> {
    try {
      const result = await client.call<{ job: JobSnapshot }>('plugins.packages.update', {
        pluginId,
        ...(version ? { version } : {}),
      });
      pendingUpdates.set(result.job.id, { pluginId, version });
      upsertJob(result.job);
    } catch (failure) {
      updatesError.value = installFailure(failure).message;
    }
  }

  async function updateAll(): Promise<void> {
    if (checkingUpdates.value) return;
    for (const entry of updates.value?.updates ?? []) {
      if (entry.latest && !entry.pinned && entry.latestStatus === 'approved') await updatePlugin(entry.id, entry.latest);
    }
  }

  async function rollback(pluginId: string, version?: string): Promise<void> {
    try {
      const result = await client.call<{ job: JobSnapshot }>('plugins.packages.rollback', {
        pluginId,
        ...(version ? { version } : {}),
      });
      upsertJob(result.job);
    } catch (failure) {
      versionsError.value = { ...versionsError.value, [pluginId]: installFailure(failure).message };
    }
  }

  async function cancelJob(jobId: string): Promise<void> {
    try {
      await client.call('plugins.jobs.cancel', { jobId });
    } catch (failure) {
      updatesError.value = errorMessage(failure);
    }
  }

  async function loadVersions(pluginId: string): Promise<void> {
    const request = (versionRequests.get(pluginId) ?? 0) + 1;
    versionRequests.set(pluginId, request);
    const previous = versions.value[pluginId];
    if (previous) versions.value = { ...versions.value, [pluginId]: { ...previous, available: previous.available.map((entry) => ({ ...entry, liveStatus:'unknown' })) } };
    try {
      const result = await client.call<Partial<PluginVersions>>('plugins.packages.versions', { pluginId });
      if (versionRequests.get(pluginId) !== request) return;
      // Normalize at the boundary: a partial host response must not leave
      // consumers reading `.length` of a missing list.
      const normalized: PluginVersions = {
        ...result,
        id: result.id ?? pluginId,
        retained: Array.isArray(result.retained) ? result.retained : [],
        available: Array.isArray(result.available) ? result.available : [],
      };
      versions.value = { ...versions.value, [pluginId]: normalized };
      const rest = { ...versionsError.value };
      delete rest[pluginId];
      versionsError.value = rest;
    } catch (failure) {
      if (versionRequests.get(pluginId) !== request) return;
      versionsError.value = { ...versionsError.value, [pluginId]: installFailure(failure).message };
    }
  }

  async function setPolicy(pluginId: string, patch: Partial<UpdatePolicy>): Promise<void> {
    try {
      const result = await client.call<{ receipt: InstallReceipt }>('plugins.packages.policy', {
        pluginId,
        ...patch,
      });
      receipts.value = { ...receipts.value, [pluginId]: result.receipt };
    } catch (failure) {
      versionsError.value = { ...versionsError.value, [pluginId]: installFailure(failure).message };
    }
  }

  return {
    ...registry,
    jobs,
    activeJobs,
    receipts,
    updates,
    updateCount,
    checkingUpdates,
    updatesError,
    versions,
    versionsError,
    review,
    initialize,
    loadReceipts,
    loadJobs,
    loadUpdates,
    checkUpdates,
    reviewSpec,
    confirmReview,
    closeReview,
    updatePlugin,
    updateAll,
    rollback,
    cancelJob,
    loadVersions,
    setPolicy,
  };
});
