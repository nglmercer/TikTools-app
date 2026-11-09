<script setup lang="ts">
import DiagnosticStatusAction from "@/shared/diagnostics/DiagnosticStatusAction.vue";
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
const lastSynced = computed(() => {
  const at = sync.status?.lastSyncedAt;
  if (at == null) return null;
  return new Date(at * 1000).toLocaleString();
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
      <BaseSwitch
        v-model="enabled"
        :disabled="sync.working || status == null"
      >
        {{ t('sync.toggle') }}
      </BaseSwitch>
      <template v-if="status != null && enabled">
        <p>{{ t('sync.pending', { count: status.pending }) }}</p>
        <p v-if="lastSynced != null">
          {{ t('sync.lastSynced', { when: lastSynced }) }}
        </p>
        <BaseAlert
          v-if="conflicts.length > 0"
          tone="warning"
          :title="t('sync.conflicts', { count: conflicts.length })"
        />
        <ul v-if="conflicts.length > 0">
          <li
            v-for="key in conflicts"
            :key="key"
          >
            {{ key }}
            <BaseButton
              :disabled="sync.working"
              @click="sync.resolve(key, true)"
            >
              {{ t('sync.keepMine') }}
            </BaseButton>
            <BaseButton
              :disabled="sync.working"
              @click="sync.resolve(key, false)"
            >
              {{ t('sync.useCloud') }}
            </BaseButton>
          </li>
        </ul>
        <BaseButton
          variant="primary"
          :disabled="sync.working"
          @click="sync.syncNow()"
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
