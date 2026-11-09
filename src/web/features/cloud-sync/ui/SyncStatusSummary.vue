<script setup lang="ts">
import BaseStatus from '@/shared/ui/status/BaseStatus.vue';
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { SyncStatus } from '@/entities/cloud-sync/api/sync-types.ts';
const props = defineProps<{ status: SyncStatus }>();
const { t, locale } = useI18n();
const lastSynced = computed(() => props.status.lastSyncedAt == null ? null : new Date(props.status.lastSyncedAt * 1000).toLocaleString(locale.value));
</script>
<template>
  <section
    class="sync-summary"
    :aria-label="t('formsDesign.syncStatus')"
  >
    <p role="status">
      <BaseStatus
        :label="t(status.syncing ? 'formsDesign.syncing' : status.enabled ? 'formsDesign.syncEnabled' : 'sync.disabled')"
        :tone="status.syncing ? 'info' : 'neutral'"
      />
    </p>
    <dl v-if="status.enabled">
      <div><dt>{{ t('formsDesign.pending') }}</dt><dd>{{ status.pending }}</dd></div>
      <div><dt>{{ t('formsDesign.lastSynced') }}</dt><dd>{{ lastSynced ?? t('formsDesign.notSynced') }}</dd></div>
    </dl>
  </section>
</template>
<style scoped>
.sync-summary { padding: var(--space-5) 0; }
.sync-summary p { margin: 0 0 var(--space-4); }
dl { display: flex; flex-wrap: wrap; gap: var(--space-5) var(--space-7); margin: 0; }
dt { color: var(--color-text-muted); font-size: var(--font-size-sm); }
dd { margin: var(--space-2) 0 0; font-variant-numeric: tabular-nums; }
</style>
