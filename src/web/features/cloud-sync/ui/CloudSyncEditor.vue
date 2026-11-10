<script setup lang="ts">
import DiagnosticStatusAction from "@/shared/diagnostics/DiagnosticStatusAction.vue";
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useSyncStore } from '@/entities/cloud-sync/model/sync-store.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';

const { t } = useI18n();
const account = useAccountStore();
const sync = useSyncStore();
const client = useControlClient();
type RecoveryValue = { state: 'absent' | 'deleted' | 'selected'; version: string | null; enabled: boolean | null };
type RecoveryReview = { handle: string; key: string; base: RecoveryValue; original: RecoveryValue; local: RecoveryValue; cloud: RecoveryValue };
const uncertain = ref<string[]>([]);
const review = ref<RecoveryReview | null>(null);
const recoveryBusy = ref(false);
const recoveryError = ref('');
const newerNeedsReview = ref(false);
const deferredCount = ref<number | null>(null);
type V2State = 'syncDisabled' | 'signInRequired' | 'backendNotSupported' | 'backendCompatible';
const v2State = ref<V2State | null>(null);
const v2Busy = ref(false);
const v2Error = ref(false);
const v2Selected = ref(false);
const protocolReady = ref(false);
const protocolLoading = ref(false);
type V2Preview = { selections: number; pending: number; conflicts: number; retainedV1Only: number; migrationAvailable: boolean };
const migrationPreview = ref<V2Preview | null>(null);
type V2Migration = { selections: number; pending: number; conflicts: number; retainedV1Only: number; completed: boolean; policyActivated: boolean };
const migrationResult = ref<V2Migration | null>(null);
type V2Result = { pushed: number; pulled: number; workspacePending: number; availableRecords: number; conflicts: string[]; moreQueued: boolean };
const v2Result = ref<V2Result | null>(null);
type V2ConflictTarget = { collection: string; key: string };
type V2ConflictValue = { state: 'absent' | 'deleted' | 'present'; revision: number | null; value: unknown; version: string | null; enabled: boolean | null };
type V2ConflictReview = { handle: string; collection: string; key: string; base: V2ConflictValue; local: V2ConflictValue; cloud: V2ConflictValue; hasNewer: boolean };
const v2Conflicts = ref<V2ConflictTarget[]>([]);
const v2ConflictReview = ref<V2ConflictReview | null>(null);
const v2ConflictBusy = ref(false);
const v2ConflictError = ref('');
const v2ConflictResolved = ref(false);
let requestGeneration = 0;
let previewGeneration = 0;
function validRecoveryValue(value: unknown): value is RecoveryValue {
  if (value == null || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  if (candidate.state === 'absent' || candidate.state === 'deleted') {
    return candidate.version === null && candidate.enabled === null;
  }
  return candidate.state === 'selected' && typeof candidate.version === 'string'
    && candidate.version.length > 0 && typeof candidate.enabled === 'boolean';
}

function validRecoveryReview(value: unknown, key: string): value is RecoveryReview {
  if (value == null || typeof value !== 'object') return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.handle === 'string' && /^[0-9a-f]{64}$/.test(candidate.handle)
    && candidate.key === key && validRecoveryValue(candidate.base)
    && validRecoveryValue(candidate.original) && validRecoveryValue(candidate.local)
    && validRecoveryValue(candidate.cloud);
}

function recoveryValueText(value: RecoveryValue): string {
  if (value.state === 'absent') return t('sync.recoveryAbsent');
  if (value.state === 'deleted') return t('sync.recoveryDeleted');
  return t('sync.recoverySelected', {
    version: value.version,
    status: t(value.enabled ? 'sync.recoveryEnabled' : 'sync.recoveryDisabled'),
  });
}

function validV2Value(value: unknown): value is V2ConflictValue {
  if (value == null || typeof value !== 'object' || Array.isArray(value)) return false;
  const candidate = value as Record<string, unknown>;
  if (candidate.state !== 'absent' && candidate.state !== 'deleted' && candidate.state !== 'present') return false;
  if (candidate.revision !== null && !Number.isSafeInteger(candidate.revision)) return false;
  if (candidate.version !== null && typeof candidate.version !== 'string') return false;
  if (candidate.enabled !== null && typeof candidate.enabled !== 'boolean') return false;
  return true;
}

function validV2Review(value: unknown, collection: string, key: string): value is V2ConflictReview {
  if (value == null || typeof value !== 'object' || Array.isArray(value)) return false;
  const candidate = value as Record<string, unknown>;
  return typeof candidate.handle === 'string' && /^[0-9a-f]{64}$/.test(candidate.handle)
    && candidate.collection === collection && candidate.key === key
    && typeof candidate.hasNewer === 'boolean' && validV2Value(candidate.base)
    && validV2Value(candidate.local) && validV2Value(candidate.cloud);
}

function v2ConflictValueText(value: V2ConflictValue): string {
  if (value.state === 'absent') return t('sync.recoveryAbsent');
  if (value.state === 'deleted') return t('sync.recoveryDeleted');
  if (typeof value.version === 'string' && typeof value.enabled === 'boolean') {
    return t('sync.recoverySelected', {
      version: value.version,
      status: t(value.enabled ? 'sync.recoveryEnabled' : 'sync.recoveryDisabled'),
    });
  }
  const preview = value.value == null ? '' : JSON.stringify(value.value).slice(0, 120);
  return `${t('sync.v2.conflictPresent')}${value.revision != null ? ` (#${value.revision})` : ''}${preview ? `: ${preview}` : ''}`;
}

async function refreshUncertain(): Promise<void> {
  const generation = requestGeneration;
  if (!signedIn.value || !enabled.value) return;
  try {
    const result = await client.call<unknown>('sync.uncertain.list', {});
    if (!Array.isArray(result) || !result.every((key) => typeof key === 'string' && key.length > 0)) throw new Error(t('sync.recoveryError'));
    if (generation === requestGeneration) uncertain.value = result;
  } catch (failure) {
    if (generation === requestGeneration) recoveryError.value = errorMessage(failure);
  }
}
async function reviewUncertain(key: string): Promise<void> {
  if (recoveryBusy.value) return;
  const generation = requestGeneration;
  recoveryBusy.value = true; recoveryError.value = ''; review.value = null; newerNeedsReview.value = false;
  try {
    const result = await client.call<unknown>('sync.uncertain.review', { key });
    if (!validRecoveryReview(result, key)) throw new Error(t('sync.recoveryError'));
    if (generation === requestGeneration) review.value = result;
  } catch (failure) {
    if (generation === requestGeneration) recoveryError.value = errorMessage(failure);
  } finally { if (generation === requestGeneration) recoveryBusy.value = false; }
}
async function resolveUncertain(choice: 'keepLocal' | 'takeCloud' | 'takeCloudIncludingNewer'): Promise<void> {
  if (!review.value || recoveryBusy.value) return;
  const generation = requestGeneration;
  recoveryBusy.value = true; recoveryError.value = '';
  try {
    const outcome = await client.call<string>('sync.uncertain.resolve', { handle: review.value.handle, choice });
    if (generation !== requestGeneration) return;
    if (outcome === 'newerEditNeedsReview') newerNeedsReview.value = true;
    else if (outcome === 'resolved') { review.value = null; newerNeedsReview.value = false; await refreshUncertain(); await sync.refresh(); }
    else throw new Error(t('sync.recoveryError'));
  } catch (failure) {
    if (generation === requestGeneration) {
      review.value = null;
      newerNeedsReview.value = false;
      recoveryError.value = errorMessage(failure);
      await refreshUncertain();
    }
  } finally { if (generation === requestGeneration) recoveryBusy.value = false; }
}

const signedIn = computed(() => account.status?.state === 'signed-in' && !account.working && !account.loading);
const status = computed(() => sync.status);
const conflicts = computed(() => sync.status?.conflicts ?? []);
const enabled = computed({
  get: () => sync.status?.enabled ?? false,
  set: (value: boolean) => {
    void sync.setEnabled(value);
  },
});
const lastSynced = computed(() => {
  const at = sync.status?.lastSyncedAt;
  if (at == null) return null;
  return new Date(at * 1000).toLocaleString();
});

async function synchronize(): Promise<void> {
  const generation = requestGeneration;
  deferredCount.value = null;
  const result = await sync.syncNow();
  if (generation !== requestGeneration || !result) return;
  if (result.applicationDeferred === true && Number.isSafeInteger(result.skipped) && result.skipped > 0) {
    deferredCount.value = result.skipped;
  }
}

async function checkV2(): Promise<void> {
  if (v2Busy.value) return;
  const generation = requestGeneration;
  v2Busy.value = true; v2Error.value = false; v2State.value = null; v2Result.value = null;
  try {
    const result = await client.call<unknown>('sync.v2.discover');
    if (generation !== requestGeneration) return;
    if (!result || typeof result !== 'object' || Array.isArray(result)) throw new Error('Invalid compatibility response');
    const value = result as Record<string, unknown>;
    const states: V2State[] = ['syncDisabled', 'signInRequired', 'backendNotSupported', 'backendCompatible'];
    if (typeof value['state'] !== 'string' || !states.includes(value['state'] as V2State) || value['desktopSyncAvailable'] !== false) throw new Error('Invalid compatibility response');
    v2State.value = value['state'] as V2State;
    if (value['selected'] !== undefined && typeof value['selected'] !== 'boolean') throw new Error('Invalid protocol response');
    if (v2State.value === 'backendCompatible' && value['selected'] !== undefined && value['selected'] !== v2Selected.value) throw new Error('Protocol changed');
  } catch {
    if (generation === requestGeneration) v2Error.value = true;
  } finally {
    if (generation === requestGeneration) v2Busy.value = false;
  }
}

async function loadProtocol(): Promise<void> {
  if (!signedIn.value || !enabled.value || protocolLoading.value) return;
  const generation = requestGeneration;
  protocolLoading.value = true; v2Error.value = false;
  try {
    const selected = await client.call<unknown>('sync.v2.selected', {});
    if (generation !== requestGeneration) return;
    if (typeof selected !== 'boolean') throw new Error('Invalid protocol response');
    v2Selected.value = selected; protocolReady.value = true;
    if (!selected) void refreshUncertain();
    else void refreshV2Conflicts();
  } catch {
    if (generation === requestGeneration) v2Error.value = true;
  } finally {
    if (generation === requestGeneration) protocolLoading.value = false;
  }
}

async function synchronizeV2(): Promise<void> {
  if (!v2Selected.value || v2Busy.value || sync.working) return;
  const generation = requestGeneration;
  v2Busy.value = true; v2Error.value = false; v2Result.value = null;
  try {
    const response = await client.call<unknown>('sync.v2.now', {});
    if (generation !== requestGeneration) return;
    if (!response || typeof response !== 'object' || Array.isArray(response)) throw new Error('Invalid sync response');
    const value = response as Record<string, unknown>;
    if (!['pushed', 'pulled', 'workspacePending', 'availableRecords'].every((key) => Number.isSafeInteger(value[key]) && (value[key] as number) >= 0) ||
      !Array.isArray(value['conflicts']) || !value['conflicts'].every((item: unknown) => typeof item === 'string') || typeof value['moreQueued'] !== 'boolean') throw new Error('Invalid sync response');
    v2Result.value = value as unknown as V2Result;
    if (generation === requestGeneration) void refreshV2Conflicts();
  } catch {
    if (generation === requestGeneration) v2Error.value = true;
  } finally {
    if (generation === requestGeneration) v2Busy.value = false;
  }
}

async function refreshV2Conflicts(): Promise<void> {
  const generation = requestGeneration;
  if (!signedIn.value || !enabled.value || !v2Selected.value) return;
  try {
    const result = await client.call<unknown>('sync.v2.conflict.list', {});
    if (generation !== requestGeneration) return;
    if (!Array.isArray(result) || !result.every((item) => item != null && typeof item === 'object' && !Array.isArray(item) && typeof (item as Record<string, unknown>).collection === 'string' && typeof (item as Record<string, unknown>).key === 'string')) throw new Error(t('sync.recoveryError'));
    v2Conflicts.value = result as unknown as V2ConflictTarget[];
  } catch (failure) {
    if (generation === requestGeneration) v2ConflictError.value = errorMessage(failure);
  }
}

async function reviewV2Conflict(collection: string, key: string): Promise<void> {
  if (v2ConflictBusy.value) return;
  const generation = requestGeneration;
  v2ConflictBusy.value = true; v2ConflictError.value = ''; v2ConflictReview.value = null; v2ConflictResolved.value = false;
  try {
    const result = await client.call<unknown>('sync.v2.conflict.review', { collection, key });
    if (generation !== requestGeneration) return;
    if (!validV2Review(result, collection, key)) throw new Error(t('sync.recoveryError'));
    v2ConflictReview.value = result;
  } catch (failure) {
    if (generation === requestGeneration) v2ConflictError.value = errorMessage(failure);
  } finally {
    if (generation === requestGeneration) v2ConflictBusy.value = false;
  }
}

async function resolveV2Conflict(choice: 'keepLocal' | 'takeCloud'): Promise<void> {
  if (!v2ConflictReview.value || v2ConflictBusy.value) return;
  const generation = requestGeneration;
  v2ConflictBusy.value = true; v2ConflictError.value = '';
  try {
    const outcome = await client.call<string>('sync.v2.conflict.resolve', { handle: v2ConflictReview.value.handle, choice });
    if (generation !== requestGeneration) return;
    if (outcome !== 'resolved') throw new Error(t('sync.recoveryError'));
    v2ConflictReview.value = null; v2ConflictResolved.value = true; await refreshV2Conflicts();
  } catch (failure) {
    if (generation === requestGeneration) {
      v2ConflictReview.value = null;
      v2ConflictError.value = errorMessage(failure);
      await refreshV2Conflicts();
    }
  } finally {
    if (generation === requestGeneration) v2ConflictBusy.value = false;
  }
}

async function previewMigration(): Promise<void> {
  if (!protocolReady.value || v2Selected.value || v2Busy.value || sync.working) return;
  const generation = requestGeneration;
  const previewTicket = ++previewGeneration;
  v2Busy.value = true; v2Error.value = false; migrationPreview.value = null; migrationResult.value = null;
  try {
    const result = await client.call<unknown>('sync.v2.preview', {});
    if (generation !== requestGeneration || previewTicket !== previewGeneration) return;
    if (!result || typeof result !== 'object' || Array.isArray(result)) throw new Error('Invalid preview');
    const value = result as Record<string, unknown>;
    if (typeof value['migrationAvailable'] !== 'boolean' || !['selections', 'pending', 'conflicts', 'retainedV1Only'].every((key) => Number.isSafeInteger(value[key]) && (value[key] as number) >= 0)) throw new Error('Invalid preview');
    migrationPreview.value = value as unknown as V2Preview;
  } catch {
    if (generation === requestGeneration && previewTicket === previewGeneration) v2Error.value = true;
  } finally {
    if (generation === requestGeneration) v2Busy.value = false;
  }
}

async function migrate(): Promise<void> {
  if (!protocolReady.value || v2Selected.value || v2Busy.value || sync.working || !migrationPreview.value?.migrationAvailable) return;
  const generation = requestGeneration;
  v2Busy.value = true; v2Error.value = false; migrationResult.value = null;
  try {
    const result = await client.call<unknown>('sync.v2.migrate', {});
    if (generation !== requestGeneration) return;
    if (!result || typeof result !== 'object' || Array.isArray(result)) throw new Error('Invalid migration response');
    const value = result as Record<string, unknown>;
    if (value['completed'] !== true || typeof value['policyActivated'] !== 'boolean' || !['selections', 'pending', 'conflicts', 'retainedV1Only'].every((key) => Number.isSafeInteger(value[key]) && (value[key] as number) >= 0)) throw new Error('Invalid migration response');
    migrationResult.value = value as unknown as V2Migration;
    v2Selected.value = true; protocolReady.value = true;
    await sync.refresh();
  } catch {
    if (generation === requestGeneration) v2Error.value = true;
  } finally {
    if (generation === requestGeneration) v2Busy.value = false;
  }
}

function clearReviews(): void {
  requestGeneration += 1; previewGeneration += 1; uncertain.value = []; review.value = null; recoveryError.value = ''; recoveryBusy.value = false; newerNeedsReview.value = false;
  deferredCount.value = null; v2State.value = null; v2Busy.value = false; v2Error.value = false; v2Selected.value = false; v2Result.value = null; migrationPreview.value = null; migrationResult.value = null; protocolReady.value = false; protocolLoading.value = false;
  v2Conflicts.value = []; v2ConflictReview.value = null; v2ConflictBusy.value = false; v2ConflictError.value = ''; v2ConflictResolved.value = false;
}
watch(() => [account.status, account.working, account.loading, enabled.value], () => {
  clearReviews();
  void loadProtocol();
}, { flush: 'sync' });
watch(() => sync.working, (working, previous) => { if (working) { previewGeneration += 1; migrationPreview.value = null; migrationResult.value = null; } if (previous && !working && protocolReady.value && !v2Selected.value) void refreshUncertain(); });
let unsubscribeContext: (() => void) | undefined;
let unsubscribeGap: (() => void) | undefined;
let unsubscribePlugins: (() => void)[] = [];
function invalidateContext(): void {
  clearReviews();
  sync.invalidate();
  void account.refresh();
}
onUnmounted(() => { requestGeneration += 1; previewGeneration += 1; unsubscribeContext?.(); unsubscribeGap?.(); unsubscribePlugins.forEach((unsubscribe) => unsubscribe()); });

onMounted(() => {
  unsubscribeContext = client.onTopic('cloud.context.invalidated', invalidateContext);
  unsubscribeGap = client.onGap(invalidateContext);
  unsubscribePlugins = ['plugin.started', 'plugin.stopped', 'plugin.status', 'plugin.job'].map((topic) => client.onTopic(topic, () => { previewGeneration += 1; migrationPreview.value = null; }));
  void account.ensureRestored();
  void sync.refresh();
});
</script>

<template>
  <BaseGroupBox
    v-if="signedIn"
    :title="t('sync.group')"
    :description="t('sync.description')"
  >
    <BaseAlert
      v-if="sync.loading"
      tone="info"
      :title="t('sync.loading')"
    />
    <template v-else>
      <DiagnosticStatusAction
        v-if="sync.error"
        :label="t('diagnostics.syncFailed')"
        source="sync"
      />
      <BaseSwitch
        v-model="enabled"
        :disabled="sync.working || status == null"
      >
        {{ t('sync.toggle') }}
      </BaseSwitch>
      <template v-if="status != null && enabled">
        <BaseButton :disabled="v2Busy || sync.working || !protocolReady" @click="checkV2">{{ t('sync.checkV2') }}</BaseButton>
        <p v-if="v2Selected" role="status">{{ t('sync.v2.selected') }}</p>
        <p v-if="v2State && (!v2Selected || v2State !== 'backendCompatible')" role="status">{{ t(`sync.v2.${v2State}`) }}</p>
        <p v-if="v2Error" role="alert">{{ t('sync.v2.error') }}</p>
        <BaseButton v-if="v2Error && !protocolReady" :disabled="protocolLoading" @click="loadProtocol">{{ t('sync.v2.retry') }}</BaseButton>
        <template v-if="protocolReady && v2Selected">
          <p v-if="migrationResult" role="status">{{ t('sync.v2.migrateResult', migrationResult) }} <span v-if="!migrationResult.policyActivated">{{ t('sync.v2.migratePolicyPending') }}</span></p>
          <p>{{ t('sync.v2.cachedOnly') }}</p>
          <p v-if="v2Result" role="status">{{ t('sync.v2.result', { pushed: v2Result.pushed, pulled: v2Result.pulled, pending: v2Result.workspacePending, available: v2Result.availableRecords, conflicts: v2Result.conflicts.length }) }} <span v-if="v2Result.moreQueued">{{ t('sync.v2.moreQueued') }}</span></p>
          <BaseButton variant="primary" :disabled="v2Busy || sync.working" @click="synchronizeV2">{{ t('sync.syncNow') }}</BaseButton>
          <BaseAlert v-if="v2ConflictError" tone="warning" :title="v2ConflictError" />
          <p v-if="v2ConflictResolved" role="status">{{ t('sync.v2.conflictResolved') }}</p>
          <section v-if="v2Conflicts.length" :aria-label="t('sync.v2.conflictTitle')">
            <h3>{{ t('sync.v2.conflictTitle') }}</h3>
            <p>{{ t('sync.v2.conflictDescription') }}</p>
            <ul>
              <li v-for="target in v2Conflicts" :key="`${target.collection}/${target.key}`">
                {{ target.collection }}/{{ target.key }}
                <BaseButton :disabled="v2ConflictBusy" @click="reviewV2Conflict(target.collection, target.key)">{{ t('sync.review') }}</BaseButton>
              </li>
            </ul>
            <div v-if="v2ConflictReview">
              <h4>{{ v2ConflictReview.collection }}/{{ v2ConflictReview.key }}</h4>
              <p>{{ t('sync.base') }}: {{ v2ConflictValueText(v2ConflictReview.base) }}</p>
              <p>{{ t('sync.local') }}: {{ v2ConflictValueText(v2ConflictReview.local) }}</p>
              <p>{{ t('sync.cloud') }}: {{ v2ConflictValueText(v2ConflictReview.cloud) }}</p>
              <p v-if="v2ConflictReview.hasNewer">{{ t('sync.v2.conflictNewer') }}</p>
              <BaseButton :disabled="v2ConflictBusy" @click="resolveV2Conflict('keepLocal')">{{ t('sync.keepMine') }}</BaseButton>
              <BaseButton :disabled="v2ConflictBusy" @click="resolveV2Conflict('takeCloud')">{{ t('sync.useCloud') }}</BaseButton>
            </div>
          </section>
        </template>
        <template v-else-if="protocolReady">
        <BaseButton :disabled="v2Busy || sync.working" @click="previewMigration">{{ t('sync.v2.preview') }}</BaseButton>
        <p v-if="migrationPreview" role="status">{{ t('sync.v2.previewResult', migrationPreview) }} {{ t('sync.v2.previewReady') }}</p>
        <template v-if="migrationPreview?.migrationAvailable">
          <p>{{ t('sync.v2.migrateWarning') }}</p>
          <BaseButton variant="primary" :disabled="v2Busy || sync.working" @click="migrate">{{ t('sync.v2.migrate') }}</BaseButton>
          <p v-if="migrationResult" role="status">{{ t('sync.v2.migrateResult', migrationResult) }} <span v-if="!migrationResult.policyActivated">{{ t('sync.v2.migratePolicyPending') }}</span></p>
        </template>
        <p>{{ t('sync.pending', { count: status.pending }) }}</p>
        <BaseAlert tone="info" :title="t('sync.applicationDeferred')" />
        <p v-if="deferredCount !== null" role="status">{{ t('sync.deferredCount', { count: deferredCount }) }}</p>
        <BaseAlert
          v-if="recoveryError"
          tone="warning"
          :title="recoveryError"
        />
        <section
          v-if="uncertain.length"
          :aria-label="t('sync.uncertainTitle')"
        >
          <h3>{{ t('sync.uncertainTitle') }}</h3>
          <p>{{ t('sync.uncertainDescription') }}</p>
          <ul>
            <li
              v-for="key in uncertain"
              :key="key"
            >
              {{ key }}
              <BaseButton
                :disabled="recoveryBusy"
                @click="reviewUncertain(key)"
              >
                {{ t('sync.review') }}
              </BaseButton>
            </li>
          </ul>
          <div v-if="review">
            <h4>{{ review.key }}</h4>
            <p>{{ t('sync.base') }}: {{ recoveryValueText(review.base) }}</p>
            <p>{{ t('sync.original') }}: {{ recoveryValueText(review.original) }}</p>
            <p>{{ t('sync.local') }}: {{ recoveryValueText(review.local) }}</p>
            <p>{{ t('sync.cloud') }}: {{ recoveryValueText(review.cloud) }}</p>
            <BaseAlert
              tone="info"
              :title="t('sync.recoveryUnavailable')"
            />
            <p v-if="newerNeedsReview">
              {{ t('sync.newerWarning') }}
            </p>
            <BaseButton
              disabled
              @click="resolveUncertain('keepLocal')"
            >
              {{ t('sync.keepMine') }}
            </BaseButton>
            <BaseButton
              disabled
              @click="resolveUncertain('takeCloud')"
            >
              {{ t('sync.useCloud') }}
            </BaseButton>
            <BaseButton
              v-if="newerNeedsReview"
              disabled
              @click="resolveUncertain('takeCloudIncludingNewer')"
            >
              {{ t('sync.discardNewer') }}
            </BaseButton>
          </div>
        </section>
        <p v-if="lastSynced != null">
          {{ t('sync.lastSynced', { when: lastSynced }) }}
        </p>
        <BaseAlert
          v-if="conflicts.length > 0"
          tone="warning"
          :title="t('sync.conflicts', { count: conflicts.length })"
        />
        <p v-if="conflicts.length > 0">{{ t('sync.conflictResolutionUnavailable') }}</p>
        <ul v-if="conflicts.length > 0">
          <li
            v-for="key in conflicts"
            :key="key"
          >
            {{ key }}
            <BaseButton
              disabled
              @click="sync.resolve(key, true)"
            >
              {{ t('sync.keepMine') }}
            </BaseButton>
            <BaseButton
              disabled
              @click="sync.resolve(key, false)"
            >
              {{ t('sync.useCloud') }}
            </BaseButton>
          </li>
        </ul>
        <BaseButton
          variant="primary"
          :disabled="sync.working"
          @click="synchronize"
        >
          {{ t('sync.syncNow') }}
        </BaseButton>
        </template>
      </template>
      <p v-else-if="status != null">
        {{ t('sync.disabled') }}
      </p>
    </template>
  </BaseGroupBox>
</template>
