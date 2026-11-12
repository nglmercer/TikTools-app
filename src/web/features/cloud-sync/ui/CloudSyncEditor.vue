<script setup lang="ts">
import { useSyncRecovery } from '../model/use-sync-recovery.ts';
import { useSyncV2 } from '../model/use-sync-v2.ts';
import SyncV2ConflictReview from './SyncV2ConflictReview.vue';
import SyncUncertainReview from './SyncUncertainReview.vue';
import DiagnosticStatusAction from "@/shared/diagnostics/DiagnosticStatusAction.vue";
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useSyncStore } from '@/entities/cloud-sync/model/sync-store.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import SettingsRow from '@/shared/ui/settings-row/SettingsRow.vue';

const { t } = useI18n();
const account = useAccountStore();
const sync = useSyncStore();
const client = useControlClient();
const deferredCount = ref<number | null>(null);
let requestGeneration = 0;
const context = { generation: () => requestGeneration, available: () => signedIn.value && enabled.value };
const recovery = useSyncRecovery(client, sync, context);
const { refreshUncertain } = recovery;
const protocol = useSyncV2(client, sync, { ...context, refreshUncertain });
const { v2State, v2Busy, v2Error, v2Selected, protocolReady, protocolLoading, migrationPreview, migrationResult, v2Result, v2RebuildBusy, v2Rebuilt, checkV2, loadProtocol, synchronizeV2, rebuildV2, previewMigration, migrate } = protocol;
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
  requestGeneration += 1;
  recovery.reset(); protocol.reset(); deferredCount.value = null;
}
watch(() => [account.status, account.working, account.loading, enabled.value], () => {
  clearReviews();
  void loadProtocol();
}, { flush: 'sync' });
watch(() => sync.working, (working, previous) => { if (working) { protocol.invalidatePreview(); } if (previous && !working && protocolReady.value && !v2Selected.value) void refreshUncertain(); });
let unsubscribeContext: (() => void) | undefined;
let unsubscribeGap: (() => void) | undefined;
let unsubscribePlugins: (() => void)[] = [];
function invalidateContext(): void {
  clearReviews();
  sync.invalidate();
  void account.refresh();
}
onUnmounted(() => { requestGeneration += 1; protocol.invalidatePreview(false); unsubscribeContext?.(); unsubscribeGap?.(); unsubscribePlugins.forEach((unsubscribe) => unsubscribe()); });

onMounted(() => {
  unsubscribeContext = client.onTopic('cloud.context.invalidated', invalidateContext);
  unsubscribeGap = client.onGap(invalidateContext);
  unsubscribePlugins = ['plugin.started', 'plugin.stopped', 'plugin.status', 'plugin.job'].map((topic) => client.onTopic(topic, () => { protocol.invalidatePreview(false); }));
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
      <SettingsRow
        toggle
        for="cloud-sync-enabled"
        :label="t('sync.toggle')"
        :description="t('formsDesign.syncDescription')"
      >
        <BaseSwitch
          id="cloud-sync-enabled"
          v-model="enabled"
          :aria-label="t('sync.toggle')"
          :disabled="sync.working || status == null"
        />
      </SettingsRow>
      <template v-if="status != null && enabled">
        <BaseButton
          :disabled="v2Busy || sync.working || !protocolReady"
          @click="checkV2"
        >
          {{ t('sync.checkV2') }}
        </BaseButton>
        <p
          v-if="v2Selected"
          role="status"
        >
          {{ t('sync.v2.selected') }}
        </p>
        <p
          v-if="v2State && (!v2Selected || v2State !== 'backendCompatible')"
          role="status"
        >
          {{ t(`sync.v2.${v2State}`) }}
        </p>
        <p
          v-if="v2Error"
          role="alert"
        >
          {{ t('sync.v2.error') }}
        </p>
        <BaseButton
          v-if="v2Error && !protocolReady"
          :disabled="protocolLoading"
          @click="loadProtocol"
        >
          {{ t('sync.v2.retry') }}
        </BaseButton>
        <template v-if="protocolReady && v2Selected">
          <p
            v-if="migrationResult"
            role="status"
          >
            {{ t('sync.v2.migrateResult', migrationResult) }} <span v-if="!migrationResult.policyActivated">{{ t('sync.v2.migratePolicyPending') }}</span>
          </p>
          <p>{{ t('sync.v2.cachedOnly') }}</p>
          <p
            v-if="v2Result"
            role="status"
          >
            {{ t('sync.v2.result', { pushed: v2Result.pushed, pulled: v2Result.pulled, pending: v2Result.workspacePending, available: v2Result.availableRecords, conflicts: v2Result.conflicts.length }) }} <span v-if="v2Result.moreQueued">{{ t('sync.v2.moreQueued') }}</span>
          </p>
          <BaseButton
            variant="primary"
            :disabled="v2Busy || sync.working"
            @click="synchronizeV2"
          >
            {{ t('sync.syncNow') }}
          </BaseButton>
          <BaseButton
            :disabled="v2RebuildBusy || v2Busy || sync.working"
            @click="rebuildV2"
          >
            {{ t('sync.v2.rebuild') }}
          </BaseButton>
          <p
            v-if="v2Rebuilt"
            role="status"
          >
            {{ t('sync.v2.rebuilt') }}
          </p>
          <SyncV2ConflictReview :model="protocol.conflicts" />
        </template>
        <template v-else-if="protocolReady">
          <BaseButton
            :disabled="v2Busy || sync.working"
            @click="previewMigration"
          >
            {{ t('sync.v2.preview') }}
          </BaseButton>
          <p
            v-if="migrationPreview"
            role="status"
          >
            {{ t('sync.v2.previewResult', migrationPreview) }} {{ t('sync.v2.previewReady') }}
          </p>
          <template v-if="migrationPreview?.migrationAvailable">
            <p>{{ t('sync.v2.migrateWarning') }}</p>
            <BaseButton
              variant="primary"
              :disabled="v2Busy || sync.working"
              @click="migrate"
            >
              {{ t('sync.v2.migrate') }}
            </BaseButton>
            <p
              v-if="migrationResult"
              role="status"
            >
              {{ t('sync.v2.migrateResult', migrationResult) }} <span v-if="!migrationResult.policyActivated">{{ t('sync.v2.migratePolicyPending') }}</span>
            </p>
          </template>
          <p>{{ t('sync.pending', { count: status.pending }) }}</p>
          <BaseAlert
            tone="info"
            :title="t('sync.applicationDeferred')"
          />
          <p
            v-if="deferredCount !== null"
            role="status"
          >
            {{ t('sync.deferredCount', { count: deferredCount }) }}
          </p>
          <SyncUncertainReview :model="recovery" />
          <p v-if="lastSynced != null">
            {{ t('sync.lastSynced', { when: lastSynced }) }}
          </p>
          <BaseAlert
            v-if="conflicts.length > 0"
            tone="warning"
            :title="t('sync.conflicts', { count: conflicts.length })"
          />
          <p v-if="conflicts.length > 0">
            {{ t('sync.conflictResolutionUnavailable') }}
          </p>
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
