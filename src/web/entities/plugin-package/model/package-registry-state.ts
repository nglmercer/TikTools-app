import { ref, shallowRef } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import type { RegistryDefaults, RegistryStatus, RegistryPackageView, SearchResults } from '../api/package-types.ts';
import { installFailure } from '../lib/install-error.ts';

/** Registry reads and preferences, instantiated once by the package store. */
export function createPackageRegistryState(client: Pick<ControlClient, 'call'>) {
  const registries = shallowRef<RegistryStatus[]>([]);
  const registriesLoaded = ref(false);
  const defaults = ref<RegistryDefaults | null>(null);
  const registryError = ref('');
  const searchQuery = ref('');
  const searchCategory = ref('');
  const searchResults = shallowRef<SearchResults | null>(null);
  const searching = ref(false);
  const searchError = ref('');
  const remotePackages = ref<Record<string, RegistryPackageView>>({});
  const remoteErrors = ref<Record<string, string>>({});
  const remoteRequests = new Map<string, number>();

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

  return { registries, registriesLoaded, defaults, registryError, searchQuery, searchCategory, searchResults, searching, searchError, remotePackages, remoteErrors, search, loadRemotePackage, loadRegistries, saveDefaults };
}
