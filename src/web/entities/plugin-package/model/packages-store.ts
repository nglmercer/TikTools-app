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
  RegistryRootPreview,
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

export interface RegistryAddInput {
  name: string;
  url: string;
  fingerprints: string[];
  scopes: string[];
  priority: number;
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
  const pendingUpdates = new Map<string, { pluginId: string; version?: string }>();
  let subscribed = false;
  let reviewRequest = 0;

  const activeJobs = computed(() => jobs.value.filter(isActiveJob));
  const updateCount = computed(() => updates.value?.updates.filter((entry) => entry.latest).length ?? 0);

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
      if (data?.report) updates.value = data.report;
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

  async function loadUpdates(): Promise<void> {
    try {
      updates.value = await client.call<UpdateReport>('plugins.updates.list', {});
    } catch {
      updates.value = null;
    }
  }

  async function checkUpdates(): Promise<void> {
    checkingUpdates.value = true;
    updatesError.value = '';
    try {
      updates.value = await client.call<UpdateReport>('plugins.updates.check', {});
    } catch (failure) {
      updatesError.value = installFailure(failure).message;
    } finally {
      checkingUpdates.value = false;
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
    for (const entry of updates.value?.updates ?? []) {
      if (entry.latest && !entry.pinned) await updatePlugin(entry.id, entry.latest);
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
    try {
      const result = await client.call<Partial<PluginVersions>>('plugins.packages.versions', { pluginId });
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
    try {
      const view = await client.call<RegistryPackageView>('registry.package', { id, registry });
      remotePackages.value = { ...remotePackages.value, [key]: view };
      const rest = { ...remoteErrors.value };
      delete rest[key];
      remoteErrors.value = rest;
    } catch (failure) {
      remoteErrors.value = { ...remoteErrors.value, [key]: installFailure(failure).message };
    }
  }

  async function loadRegistries(): Promise<void> {
    try {
      const [list, current] = await Promise.all([
        client.call<{ registries: RegistryStatus[] }>('registry.list', {}),
        client.call<RegistryDefaults>('registry.defaults.get', {}),
      ]);
      registries.value = list.registries;
      defaults.value = current;
      registryError.value = '';
    } catch (failure) {
      registryError.value = installFailure(failure).message;
    }
  }

  async function refreshRegistries(name?: string): Promise<void> {
    try {
      await client.call('registry.refresh', name ? { name } : {});
      await loadRegistries();
    } catch (failure) {
      registryError.value = installFailure(failure).message;
    }
  }

  async function previewRegistry(url: string): Promise<RegistryRootPreview | null> {
    try {
      registryError.value = '';
      return await client.call<RegistryRootPreview>('registry.root', { url });
    } catch (failure) {
      registryError.value = installFailure(failure).message;
      return null;
    }
  }

  async function addRegistry(input: RegistryAddInput): Promise<boolean> {
    try {
      registryError.value = '';
      await client.call('registry.add', { ...input });
      await loadRegistries();
      return true;
    } catch (failure) {
      registryError.value = installFailure(failure).message;
      return false;
    }
  }

  async function removeRegistry(name: string): Promise<void> {
    try {
      await client.call('registry.remove', { name });
      await loadRegistries();
    } catch (failure) {
      registryError.value = installFailure(failure).message;
    }
  }

  async function setRegistryEnabled(name: string, enabled: boolean): Promise<void> {
    try {
      await client.call('registry.enable', { name, enabled });
      await loadRegistries();
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
    refreshRegistries,
    previewRegistry,
    addRegistry,
    removeRegistry,
    setRegistryEnabled,
    saveDefaults,
  };
});
