import { ref } from 'vue';
import { errorMessage } from '@/shared/api/control-client.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';
import type { SyncRecoveryContext } from './recovery-context.ts';
import type { V2State, V2Preview, V2Migration, V2Result } from './recovery-contracts.ts';
import { useSyncV2Conflicts } from './use-sync-v2-conflicts.ts';

export function useSyncV2(client: ControlClient, sync: { working: boolean; refresh(): Promise<void> }, context: SyncRecoveryContext & { refreshUncertain(): Promise<void> }) {
  const v2State = ref<V2State | null>(null);
  const v2Busy = ref(false);
  const v2Error = ref(false);
  const v2ErrorMessage = ref('');
  const v2Selected = ref(false);
  const protocolReady = ref(false);
  const protocolLoading = ref(false);
  const migrationPreview = ref<V2Preview | null>(null);
  const migrationResult = ref<V2Migration | null>(null);
  const v2Result = ref<V2Result | null>(null);
  const v2RebuildBusy = ref(false);
  const v2Rebuilt = ref(false);
  let previewGeneration = 0;
  const conflicts = useSyncV2Conflicts(client, {
    generation: context.generation,
    available: () => context.available() && v2Selected.value,
  });
  const { refreshV2Conflicts } = conflicts;
  async function checkV2(): Promise<void> {
    if (v2Busy.value) return;
    const generation = context.generation();
    v2Busy.value = true; v2Error.value = false; v2ErrorMessage.value = '';  v2State.value = null; v2Result.value = null;
    try {
      const result = await client.call<unknown>('sync.v2.discover');
      if (generation !== context.generation()) return;
      if (!result || typeof result !== 'object' || Array.isArray(result)) throw new Error('Invalid compatibility response');
      const value = result as Record<string, unknown>;
      const states: V2State[] = ['syncDisabled', 'signInRequired', 'backendNotSupported', 'backendCompatible'];
      if (typeof value['state'] !== 'string' || !states.includes(value['state'] as V2State) || value['desktopSyncAvailable'] !== false) throw new Error('Invalid compatibility response');
      v2State.value = value['state'] as V2State;
      if (value['selected'] !== undefined && typeof value['selected'] !== 'boolean') throw new Error('Invalid protocol response');
      if (v2State.value === 'backendCompatible' && value['selected'] !== undefined && value['selected'] !== v2Selected.value) throw new Error('Protocol changed');
    } catch (failure) {
      if (generation === context.generation()) { v2Error.value = true; v2ErrorMessage.value = errorMessage(failure); }
    } finally {
      if (generation === context.generation()) v2Busy.value = false;
    }
  }

  async function loadProtocol(): Promise<void> {
    if (!context.available() || protocolLoading.value) return;
    const generation = context.generation();
    protocolLoading.value = true; v2Error.value = false; v2ErrorMessage.value = '';
    try {
      const selected = await client.call<unknown>('sync.v2.selected', {});
      if (generation !== context.generation()) return;
      if (typeof selected !== 'boolean') throw new Error('Invalid protocol response');
      v2Selected.value = selected; protocolReady.value = true;
      if (!selected) void context.refreshUncertain();
      else void refreshV2Conflicts();
    } catch (failure) {
      if (generation === context.generation()) { v2Error.value = true; v2ErrorMessage.value = errorMessage(failure); }
    } finally {
      if (generation === context.generation()) protocolLoading.value = false;
    }
  }

  async function synchronizeV2(): Promise<void> {
    if (!v2Selected.value || v2Busy.value || sync.working) return;
    const generation = context.generation();
    v2Busy.value = true; v2Error.value = false; v2ErrorMessage.value = '';  v2Result.value = null;
    try {
      const response = await client.call<unknown>('sync.v2.now', {});
      if (generation !== context.generation()) return;
      if (!response || typeof response !== 'object' || Array.isArray(response)) throw new Error('Invalid sync response');
      const value = response as Record<string, unknown>;
      if (!['pushed', 'pulled', 'workspacePending', 'availableRecords'].every((key) => Number.isSafeInteger(value[key]) && (value[key] as number) >= 0) ||
        !Array.isArray(value['conflicts']) || !value['conflicts'].every((item: unknown) => typeof item === 'string') || typeof value['moreQueued'] !== 'boolean') throw new Error('Invalid sync response');
      v2Result.value = value as unknown as V2Result;
      if (generation === context.generation()) void refreshV2Conflicts();
    } catch (failure) {
      if (generation === context.generation()) { v2Error.value = true; v2ErrorMessage.value = errorMessage(failure); }
    } finally {
      if (generation === context.generation()) v2Busy.value = false;
    }
  }

  async function rebuildV2(): Promise<void> {
    if (!v2Selected.value || v2RebuildBusy.value || v2Busy.value || sync.working) return;
    const generation = context.generation();
    v2RebuildBusy.value = true; v2Error.value = false; v2ErrorMessage.value = '';  v2Rebuilt.value = false;
    try {
      await client.call<unknown>('sync.v2.rebuild', {});
      if (generation !== context.generation()) return;
      v2Rebuilt.value = true; await refreshV2Conflicts();
    } catch (failure) {
      if (generation === context.generation()) { v2Error.value = true; v2ErrorMessage.value = errorMessage(failure); }
    } finally {
      if (generation === context.generation()) v2RebuildBusy.value = false;
    }
  }

  async function previewMigration(): Promise<void> {
    if (!protocolReady.value || v2Selected.value || v2Busy.value || sync.working) return;
    const generation = context.generation();
    const previewTicket = ++previewGeneration;
    v2Busy.value = true; v2Error.value = false; v2ErrorMessage.value = '';  migrationPreview.value = null; migrationResult.value = null;
    try {
      const result = await client.call<unknown>('sync.v2.preview', {});
      if (generation !== context.generation() || previewTicket !== previewGeneration) return;
      if (!result || typeof result !== 'object' || Array.isArray(result)) throw new Error('Invalid preview');
      const value = result as Record<string, unknown>;
      if (typeof value['migrationAvailable'] !== 'boolean' || !['selections', 'pending', 'conflicts', 'retainedV1Only'].every((key) => Number.isSafeInteger(value[key]) && (value[key] as number) >= 0)) throw new Error('Invalid preview');
      migrationPreview.value = value as unknown as V2Preview;
    } catch (failure) {
      if (generation === context.generation() && previewTicket === previewGeneration) { v2Error.value = true; v2ErrorMessage.value = errorMessage(failure); }
    } finally {
      if (generation === context.generation()) v2Busy.value = false;
    }
  }

  async function migrate(): Promise<void> {
    if (!protocolReady.value || v2Selected.value || v2Busy.value || sync.working || !migrationPreview.value?.migrationAvailable) return;
    const generation = context.generation();
    v2Busy.value = true; v2Error.value = false; v2ErrorMessage.value = '';  migrationResult.value = null;
    try {
      const result = await client.call<unknown>('sync.v2.migrate', {});
      if (generation !== context.generation()) return;
      if (!result || typeof result !== 'object' || Array.isArray(result)) throw new Error('Invalid migration response');
      const value = result as Record<string, unknown>;
      if (value['completed'] !== true || typeof value['policyActivated'] !== 'boolean' || !['selections', 'pending', 'conflicts', 'retainedV1Only'].every((key) => Number.isSafeInteger(value[key]) && (value[key] as number) >= 0)) throw new Error('Invalid migration response');
      migrationResult.value = value as unknown as V2Migration;
      v2Selected.value = true; protocolReady.value = true;
      await sync.refresh();
    } catch (failure) {
      if (generation === context.generation()) { v2Error.value = true; v2ErrorMessage.value = errorMessage(failure); }
    } finally {
      if (generation === context.generation()) v2Busy.value = false;
    }
  }


  function invalidatePreview(clearResult = true): void {
    previewGeneration += 1; migrationPreview.value = null;
    if (clearResult) migrationResult.value = null;
  }
  function reset(): void {
    invalidatePreview();
    v2State.value = null; v2Busy.value = false; v2Error.value = false; v2ErrorMessage.value = '';  v2Selected.value = false; v2Result.value = null;
    protocolReady.value = false; protocolLoading.value = false; v2RebuildBusy.value = false; v2Rebuilt.value = false;
    conflicts.reset();
  }
  return { v2State, v2Busy, v2Error, v2ErrorMessage, v2Selected, protocolReady, protocolLoading, migrationPreview, migrationResult, v2Result, v2RebuildBusy, v2Rebuilt, conflicts, checkV2, loadProtocol, synchronizeV2, rebuildV2, previewMigration, migrate, invalidatePreview, reset };
}
