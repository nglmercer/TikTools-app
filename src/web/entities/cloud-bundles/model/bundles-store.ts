import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';
import { packagePreflight, uploadAsset, type AssetRecord, type PackagePreflight } from '../api/bundles-api.ts';

export const MAX_IMAGE_BYTES = 512 * 1024;

/**
 * Cloud assets and package preflight. A preflight is read-only: applying a package
 * is a separate client step that this store does not perform.
 */
export const useBundlesStore = defineStore('cloud-bundles', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();
  const asset = ref<AssetRecord | null>(null);
  const preflight = ref<PackagePreflight | null>(null);
  const packageId = ref<string | null>(null);
  const working = ref(false);
  const error = ref<string | null>(null);
  let generation = 0;

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
      error.value = errorMessage(failure);
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
      if (ticket === generation) error.value = errorMessage(failure);
      return false;
    } finally {
      if (ticket === generation) working.value = false;
    }
  }

  return { asset, preflight, packageId, working, error, available, upload, checkPackage };
});
