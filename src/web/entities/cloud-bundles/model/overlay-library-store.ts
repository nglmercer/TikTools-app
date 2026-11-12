import { defineStore } from 'pinia';
import { computed, ref, shallowRef } from 'vue';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { lifecycleFailure } from '@/shared/api/cloud-lifecycle.ts';
import { createPagedList } from '@/shared/lib/paged-list.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { useCloudContext } from '@/entities/cloud-account/model/cloud-context.ts';
import {
  deleteOverlay,
  listOverlays,
  listRevisions,
  readRevision,
  type OverlayRevisionRecord,
  type OverlayRevisionSummary,
  type OverlaySummary,
} from '../api/lifecycle-api.ts';
import { useItemFailure } from './lifecycle-failure.ts';

/**
 * The owner's Cloud overlays: the list, the revisions of one overlay, the document of a revision
 * (to view it, or to compare two) and the delete. Revisions and documents belong to the overlay
 * that is open: opening another one, going back or a change of account makes every earlier reply
 * stale. A delete the Cloud refuses because a package pins the overlay is explained by `failure`.
 */
export const useCloudOverlayLibraryStore = defineStore('cloud-overlay-library', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();
  const overlays = createPagedList<OverlaySummary>({
    key: (overlay) => overlay.id,
    fetch: (after) => listOverlays(client, after),
  });
  const openId = ref<string | null>(null);
  const revisions = createPagedList<OverlayRevisionSummary>({
    key: (revision) => String(revision.revision),
    fetch: (after) => {
      const id = openId.value;
      if (id === null) return Promise.resolve({ items: [], nextCursor: null });
      return listRevisions(client, id, after);
    },
  });
  const documents = shallowRef<Readonly<Record<number, OverlayRevisionRecord>>>({});
  const documentsLoading = ref(0);
  const documentFailed = ref(false);
  const item = useItemFailure<'delete'>();
  const busyId = ref<string | null>(null);
  const cloudUnsupported = ref(false);
  let epoch = 0;
  let openEpoch = 0;

  const cloud = useCloudContext(() => {
    epoch += 1;
    openEpoch += 1;
    overlays.reset();
    revisions.reset();
    openId.value = null;
    documents.value = {};
    documentsLoading.value = 0;
    documentFailed.value = false;
    item.clear();
    busyId.value = null;
    cloudUnsupported.value = false;
  });
  void cloud;

  const available = computed(() => (cloudUnsupported.value ? false : capabilities.supports('bundles.overlays.list')));
  const canDelete = computed(() => capabilities.supports('bundles.overlays.delete') === true && !cloudUnsupported.value);
  const canInspect = computed(
    () => capabilities.supports('bundles.overlays.revisions') === true
      && capabilities.supports('bundles.overlays.revision') === true
      && !cloudUnsupported.value,
  );
  const listFailure = computed(() => {
    const failure = overlays.error.value;
    return failure !== null && lifecycleFailure(failure) !== 'unsupported' ? failure : null;
  });
  const open = computed(() => overlays.items.value.find((overlay) => overlay.id === openId.value) ?? null);

  async function refresh(): Promise<void> {
    if (available.value === false) return;
    item.clear();
    await overlays.refresh();
    if (lifecycleFailure(overlays.error.value) === 'unsupported') cloudUnsupported.value = true;
  }

  /** Opens the revisions of one overlay, dropping the revisions and documents of the previous one. */
  async function openOverlay(id: string): Promise<void> {
    if (!canInspect.value) return;
    openEpoch += 1;
    revisions.reset();
    documents.value = {};
    documentsLoading.value = 0;
    documentFailed.value = false;
    openId.value = id;
    await revisions.refresh();
  }

  function closeOverlay(): void {
    openEpoch += 1;
    revisions.reset();
    documents.value = {};
    documentsLoading.value = 0;
    documentFailed.value = false;
    openId.value = null;
  }

  /** Reads one revision's document into `documents`. A reply for an overlay that is no longer open is dropped. */
  async function loadDocument(revision: number): Promise<OverlayRevisionRecord | null> {
    const id = openId.value;
    if (id === null || !canInspect.value) return null;
    const cached = documents.value[revision];
    if (cached) return cached;
    const mine = openEpoch;
    documentsLoading.value += 1;
    documentFailed.value = false;
    try {
      const record = await readRevision(client, id, revision);
      if (mine !== openEpoch) return null;
      documents.value = { ...documents.value, [revision]: record };
      return record;
    } catch {
      if (mine === openEpoch) documentFailed.value = true;
      return null;
    } finally {
      if (mine === openEpoch) documentsLoading.value -= 1;
    }
  }

  async function remove(id: string): Promise<boolean> {
    if (!canDelete.value || busyId.value !== null) return false;
    const mine = epoch;
    busyId.value = id;
    item.clear();
    try {
      await deleteOverlay(client, id);
      if (mine !== epoch) return false;
      overlays.remove(id);
      if (openId.value === id) closeOverlay();
      return true;
    } catch (failure) {
      if (mine === epoch) item.set(id, 'delete', failure);
      return false;
    } finally {
      if (mine === epoch) busyId.value = null;
    }
  }

  return {
    overlays: overlays.items,
    loading: overlays.loading,
    loaded: overlays.loaded,
    hasMore: overlays.hasMore,
    listFailure,
    openId,
    open,
    revisions: revisions.items,
    revisionsLoading: revisions.loading,
    revisionsHasMore: revisions.hasMore,
    revisionsFailure: revisions.error,
    documents,
    documentsLoading,
    documentFailed,
    failure: item.failure,
    busyId,
    available,
    canDelete,
    canInspect,
    refresh,
    loadMore: overlays.loadMore,
    loadMoreRevisions: revisions.loadMore,
    openOverlay,
    closeOverlay,
    loadDocument,
    remove,
    dismissFailure: item.clear,
  };
});
