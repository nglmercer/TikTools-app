import { defineStore } from 'pinia';
import { computed, ref, shallowRef } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { deleteGlobal, listGlobals, listTypedGlobals, replaceTypedGlobals, setGlobal } from '../api/globals-api.ts';
import { sortedGlobalEntries } from '../lib/global-key.ts';
import { textVariables, variableText, type RuntimeVariable, type RuntimeVariablesSnapshot } from '../lib/runtime-variable.ts';
import { useCapabilityStore } from '@/entities/runtime/model/capabilities-store.ts';

export const useGlobalsStore = defineStore('runtime-globals', () => {
  const client = useControlClient();
  const capabilities = useCapabilityStore();
  const globals = ref<Record<string, string>>({});
  const loading = ref(false);
  const error = ref<string | null>(null);
  const typedSnapshot = shallowRef<RuntimeVariablesSnapshot | null>(null);
  let generation = 0;
  const typedSupported = computed(() => capabilities.supports('globals.typed.list') === true && capabilities.supports('globals.typed.replace') === true);
  const variables = computed(() => typedSnapshot.value?.variables ?? textVariables(globals.value));
  const revision = computed(() => typedSnapshot.value?.revision ?? null);

  function accept(snapshot: RuntimeVariablesSnapshot): void {
    typedSnapshot.value = snapshot;
    globals.value = Object.fromEntries(Object.entries(snapshot.variables).map(([name, variable]) => [name, variableText(variable)]));
  }

  async function loadTyped(): Promise<void> {
    if (!typedSupported.value) { await load(); return; }
    const ticket = ++generation;
    loading.value = true;
    error.value = null;
    try {
      const next = await listTypedGlobals(client);
      if (ticket === generation) accept(next);
    } catch (failure) {
      if (ticket === generation) error.value = errorMessage(failure);
    } finally {
      if (ticket === generation) loading.value = false;
    }
  }

  async function load(): Promise<void> {
    if (typedSupported.value) { await loadTyped(); return; }
    const ticket = ++generation;
    loading.value = true;
    error.value = null;
    try {
      const next = await listGlobals(client);
      if (ticket === generation) { globals.value = next; typedSnapshot.value = null; }
    } catch (failure) {
      if (ticket === generation) error.value = errorMessage(failure);
    } finally {
      if (ticket === generation) loading.value = false;
    }
  }

  async function saveTyped(expectedRevision: string, next: Record<string, RuntimeVariable>): Promise<void> {
    if (!typedSupported.value) throw new Error('Typed variables are unavailable on this host');
    const ticket = ++generation;
    loading.value = false;
    error.value = null;
    try {
      const result = await replaceTypedGlobals(client, expectedRevision, next);
      if (ticket === generation) accept(result);
      else await loadTyped();
    } catch (failure) {
      // Keep the caller's reviewed revision. A retry after a conflict must
      // not silently use a fresh revision to overwrite another window.
      error.value = errorMessage(failure);
      throw failure;
    }
  }

  async function save(next: Record<string, string>): Promise<void> {
    const ticket = ++generation;
    loading.value = false;
    error.value = null;
    const current = globals.value;
    const deletions = Object.keys(current)
      .filter((key) => !Object.hasOwn(next, key))
      .sort();
    const writes = sortedGlobalEntries(next).filter(([key, value]) => current[key] !== value);
    try {
      for (const key of deletions) await deleteGlobal(client, key);
      for (const [key, value] of writes) await setGlobal(client, key, value);
      if (ticket === generation) globals.value = { ...next };
      if (typedSupported.value) await loadTyped();
    } catch (failure) {
      const message = errorMessage(failure);
      await load();
      error.value = message;
      throw failure;
    }
  }

  return { globals, variables, revision, typedSupported, loading, error, load, loadTyped, save, saveTyped };
});
