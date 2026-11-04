<script setup lang="ts">
import { computed, onMounted } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseGroupBox from '@/shared/ui/group-box/BaseGroupBox.vue';
import { useSyncStore } from '@/entities/cloud-sync/model/sync-store.ts';

const { t } = useI18n();
const sync = useSyncStore();

const status = computed(() => sync.status);
const conflicts = computed(() => sync.status?.conflicts ?? []);
const lastSynced = computed(() => {
  const at = sync.status?.lastSyncedAt;
  if (at == null) return null;
  return new Date(at * 1000).toLocaleString();
});

onMounted(() => {
  void sync.refresh();
});
</script>

<template>
  <BaseGroupBox
    :title="t('sync.group')"
    :description="t('sync.description')"
  >
    <BaseAlert
      v-if="sync.loading"
      tone="info"
      :title="t('sync.loading')"
    />
    <template v-else>
      <BaseAlert
        v-if="sync.error"
        :title="sync.error"
      />
      <template v-if="status != null && !status.enabled">
        <p>{{ t('sync.disabled') }}</p>
        <BaseButton
          variant="primary"
          :disabled="sync.working"
          @click="sync.setEnabled(true)"
        >
          {{ t('sync.enable') }}
        </BaseButton>
      </template>
      <template v-else-if="status != null">
        <p>{{ t('sync.pending', { count: status.pending }) }}</p>
        <p v-if="lastSynced != null">
          {{ t('sync.lastSynced', { when: lastSynced }) }}
        </p>
        <p v-if="!status.signedIn">
          {{ t('sync.signedOut') }}
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
        <BaseButton
          :disabled="sync.working"
          @click="sync.setEnabled(false)"
        >
          {{ t('sync.disable') }}
        </BaseButton>
      </template>
    </template>
  </BaseGroupBox>
</template>
