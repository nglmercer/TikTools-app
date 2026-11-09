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
  RegistryDefaults,
  RegistryStatus,
  RegistryPackageView,
  SearchResults,
  UpdatePolicy,
  UpdateReport,
} from '../api/package-types.ts';
import { installFailure, reviewFromError } from '../lib/install-error.ts';
import { isActiveJob } from '../lib/package-format.ts';

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

  const jobs = shallowRef<JobSnapshot[]>([]);
  const receipts = ref<Record<string, InstallReceipt>>({});
  const updates = shallowRef<UpdateReport | null>(null);
  const checkingUpdates = ref(false);
  const updatesError = ref('');
  const registries = shallowRef<RegistryStatus[]>([]);
  const registriesLoaded = ref(false);
  const defaults = ref<RegistryDefaults | null>(null);
  const registryError = ref('');
  const searchQuery = ref('');
  const searchCategory = ref('');
  const searchResults = shallowRef<SearchResults | null>(null);
  const searching = ref(false);
  const searchError = ref('');
  const versions = ref<Record<string, PluginVersions>>({});
  const versionsError = ref<Record<string, string>>({});
  const review = ref<ReviewState>(closedReview());
  const remotePackages = ref<Record<string, RegistryPackageView>>({});
  const remoteErrors = ref<Record<string, string>>({});
  const remoteRequests = new Map<string, number>();
  const versionRequests = new Map<string, number>();
  const pendingUpdates = new Map<string, { pluginId: string; version?: string }>();
  let subscribed = false;
  let reviewRequest = 0;
  let updatesRequest = 0;
  let updatesNotificationPending = false;

  const activeJobs = computed(() => jobs.value.filter(isActiveJob));
  const updateCount = computed(() => updates.value?.updates.filter((entry) => entry.latest && entry.latestStatus === 'approved' && !entry.pinned).length ?? 0);

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
      if (checkingUpdates.value) updatesNotificationPending = true;
      else void loadUpdates();
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

  function invalidateUpdateApprovals(): void {
    if (updates.value) updates.value = { ...updates.value, updates: updates.value.updates.map((entry) => ({ ...entry, currentStatus:'unknown', latestStatus:'unknown', autoApplicable:false })) };
  }

  async function loadUpdates(): Promise<void> {
    if (checkingUpdates.value) return;
    const request = ++updatesRequest;
    invalidateUpdateApprovals();
    try {
      const result = await client.call<UpdateReport>('plugins.updates.list', {});
      if (request === updatesRequest) updates.value = result;
    } catch {
      if (request === updatesRequest) updates.value = null;
    }
  }

  async function checkUpdates(): Promise<void> {
    if (checkingUpdates.value) return;
    const request = ++updatesRequest;
    checkingUpdates.value = true;
    updatesError.value = '';
    invalidateUpdateApprovals();
    try {
      const result = await client.call<UpdateReport>('plugins.updates.check', {});
      if (request === updatesRequest) updates.value = result;
    } catch (failure) {
      if (request === updatesRequest) updatesError.value = installFailure(failure).message;
    } finally {
      if (request === updatesRequest) {
        checkingUpdates.value = false;
        if (updatesNotificationPending) {
          updatesNotificationPending = false;
          void loadUpdates();
        }
      }
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

  async function search(page = 1): Promise<void> {
    searching.value = true;
    searchError.value = '';
    try {
      searchResults.value = await client.call<SearchResults>('registry.search', {
        query: searchQuery.value.trim(),
        page,
        ...(searchCategory.value ? { category: searchCategory.value } : {}),
      });
    } catch (failure) {
      searchError.value = installFailure(failure).message;
    } finally {
      searching.value = false;
    }
  }

  async function loadRemotePackage(registry: string, id: string): Promise<void> {
    const key = `${registry}/${id}`;
    const request = (remoteRequests.get(key) ?? 0) + 1;
    remoteRequests.set(key, request);
    const previous = remotePackages.value[key];
    if (previous) remotePackages.value = { ...remotePackages.value, [key]: { ...previous, liveStatuses:{} } };
    try {
      const view = await client.call<RegistryPackageView>('registry.package', { id, registry });
      if (remoteRequests.get(key) !== request) return;
      remotePackages.value = { ...remotePackages.value, [key]: view };
      const rest = { ...remoteErrors.value };
      delete rest[key];
      remoteErrors.value = rest;
    } catch (failure) {
      if (remoteRequests.get(key) !== request) return;
      remoteErrors.value = { ...remoteErrors.value, [key]: installFailure(failure).message };
    }
  }

  async function loadRegistries(): Promise<void> {
    try {
      const [list, current] = await Promise.all([
        client.call<{ registries: RegistryStatus[] }>('registry.list', {}),
        client.call<RegistryDefaults>('registry.defaults.get', {}),
      ]);
      registries.value = Array.isArray(list?.registries) ? list.registries : [];
      registriesLoaded.value = Array.isArray(list?.registries);
      defaults.value = current;
      registryError.value = '';
    } catch (failure) {
      registryError.value = installFailure(failure).message;
    }
  }

  async function saveDefaults(next: RegistryDefaults): Promise<void> {
    try {
      defaults.value = await client.call<RegistryDefaults>('registry.defaults.set', { ...next });
      registryError.value = '';
    } catch (failure) {
      registryError.value = installFailure(failure).message;
    }
  }

  return {
    jobs,
    activeJobs,
    receipts,
    updates,
    updateCount,
    checkingUpdates,
    updatesError,
    registries,
    registriesLoaded,
    defaults,
    registryError,
    searchQuery,
    searchCategory,
    searchResults,
    searching,
    searchError,
    versions,
    versionsError,
    review,
    remotePackages,
    remoteErrors,
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
    search,
    loadRemotePackage,
    loadRegistries,
    saveDefaults,
  };
});
