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
let requestGeneration = 0;
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

function clearReviews(): void {
  requestGeneration += 1; uncertain.value = []; review.value = null; recoveryError.value = ''; recoveryBusy.value = false; newerNeedsReview.value = false;
  deferredCount.value = null;
}
watch(() => [account.status, account.working, account.loading, enabled.value], () => {
  clearReviews();
  void refreshUncertain();
}, { flush: 'sync' });
watch(() => sync.working, (working, previous) => { if (previous && !working) void refreshUncertain(); });
let unsubscribeContext: (() => void) | undefined;
let unsubscribeGap: (() => void) | undefined;
function invalidateContext(): void {
  clearReviews();
  sync.invalidate();
  void account.refresh();
}
onUnmounted(() => { requestGeneration += 1; unsubscribeContext?.(); unsubscribeGap?.(); });

onMounted(() => {
  unsubscribeContext = client.onTopic('cloud.context.invalidated', invalidateContext);
  unsubscribeGap = client.onGap(invalidateContext);
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
      <p v-else-if="status != null">
        {{ t('sync.disabled') }}
      </p>
    </template>
  </BaseGroupBox>
</template>
