<script setup lang="ts">
import DiagnosticStatusAction from "@/shared/diagnostics/DiagnosticStatusAction.vue";
import SettingsRow from '@/shared/ui/settings-row/SettingsRow.vue';
import SyncStatusSummary from './SyncStatusSummary.vue';
import SyncConflictReview from './SyncConflictReview.vue';
import { computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useSyncStore } from '@/entities/cloud-sync/model/sync-store.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';

const { t } = useI18n();
const account = useAccountStore();
const sync = useSyncStore();

const signedIn = computed(() => account.status?.state === 'signed-in');
const status = computed(() => sync.status);
const conflicts = computed(() => sync.status?.conflicts ?? []);
const enabled = computed({
  get: () => sync.status?.enabled ?? false,
  set: (value: boolean) => {
    void sync.setEnabled(value);
  },
});

onMounted(() => {
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
      <template v-if="status != null">
        <SyncStatusSummary :status="status" />
        <template v-if="enabled">
          <SyncConflictReview
            :conflicts="conflicts"
            :working="sync.working"
            @resolve="sync.resolve"
          />
          <BaseButton
            variant="primary"
            :disabled="sync.working"
            @click="sync.syncNow()"
          >
            {{ t('sync.syncNow') }}
          </BaseButton>
        </template>
      </template>
    </template>
  </BaseGroupBox>
</template>
