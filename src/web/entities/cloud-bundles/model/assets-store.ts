import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { lifecycleFailure } from '@/shared/api/cloud-lifecycle.ts';
import { createPagedList } from '@/shared/lib/paged-list.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { useCloudContext } from '@/entities/cloud-account/model/cloud-context.ts';
import type { AssetRecord } from '../api/bundles-api.ts';
import { deleteAsset, listAssets, setAssetLabel } from '../api/lifecycle-api.ts';
import { useItemFailure } from './lifecycle-failure.ts';

export const MAX_LABEL_CHARS = 120;

/**
 * The owner's Cloud images: a paged list, an inline rename and a delete. Each action is offered
 * only when the host announces its method, and a Cloud that does not offer the feature (the host
 * answers `capability_unavailable`) turns the whole area off instead of showing a failure. A new
 * account, session or host drops the rows, and a slow reply from before that never comes back.
 */
export const useCloudAssetsStore = defineStore('cloud-assets', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();
  const list = createPagedList<AssetRecord>({
    key: (asset) => asset.id,
    fetch: (after) => listAssets(client, after),
  });
  const item = useItemFailure<'rename' | 'delete'>();
  const busyId = ref<string | null>(null);
  /** The Cloud instance does not offer image management. */
  const cloudUnsupported = ref(false);
  let epoch = 0;

  const cloud = useCloudContext(() => {
    epoch += 1;
    list.reset();
    item.clear();
    busyId.value = null;
    cloudUnsupported.value = false;
  });
  void cloud;

  const available = computed(() => (cloudUnsupported.value ? false : capabilities.supports('bundles.assets.list')));
  const canRename = computed(() => capabilities.supports('bundles.assets.setLabel') === true && !cloudUnsupported.value);
  const canDelete = computed(() => capabilities.supports('bundles.assets.delete') === true && !cloudUnsupported.value);
  /** Message of a list failure that is not "the Cloud does not offer this". */
  const listFailure = computed(() => {
    const failure = list.error.value;
    if (failure === null) return null;
    return lifecycleFailure(failure) === 'unsupported' ? null : failure;
  });

  async function refresh(): Promise<void> {
    if (available.value === false) return;
    item.clear();
    await list.refresh();
    if (lifecycleFailure(list.error.value) === 'unsupported') cloudUnsupported.value = true;
  }

  function loadMore(): Promise<void> {
    return list.loadMore();
  }

  /** Renames an image. An empty label clears it. Resolves `true` when the Cloud confirmed. */
  async function rename(id: string, label: string): Promise<boolean> {
    if (!canRename.value || busyId.value !== null) return false;
    const trimmed = label.trim();
    if (trimmed.length > MAX_LABEL_CHARS) return false;
    const mine = epoch;
    busyId.value = id;
    item.clear();
    try {
      const next = await setAssetLabel(client, id, trimmed.length > 0 ? trimmed : null);
      if (mine !== epoch) return false;
      list.replace(next);
      return true;
    } catch (failure) {
      if (mine === epoch) item.set(id, 'rename', failure);
      return false;
    } finally {
      if (mine === epoch) busyId.value = null;
    }
  }

  /** Deletes an image. While an overlay revision names it the Cloud refuses; `failure` explains why. */
  async function remove(id: string): Promise<boolean> {
    if (!canDelete.value || busyId.value !== null) return false;
    const mine = epoch;
    busyId.value = id;
    item.clear();
    try {
      await deleteAsset(client, id);
      if (mine !== epoch) return false;
      list.remove(id);
      return true;
    } catch (failure) {
      if (mine === epoch) item.set(id, 'delete', failure);
      return false;
    } finally {
      if (mine === epoch) busyId.value = null;
    }
  }

  return {
    assets: list.items,
    loading: list.loading,
    loaded: list.loaded,
    hasMore: list.hasMore,
    listFailure,
    failure: item.failure,
    busyId,
    available,
    canRename,
    canDelete,
    refresh,
    loadMore,
    rename,
    remove,
    dismissFailure: item.clear,
  };
});
