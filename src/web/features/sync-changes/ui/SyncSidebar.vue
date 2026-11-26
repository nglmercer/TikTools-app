<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SyncAdapter } from '@/entities/cloud-sync/api/sync-adapter.ts';
import type { SyncPanelState } from '../model/use-sync-panel.ts';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';

type SyncSidebarPanel = Pick<SyncPanelState,
  'view' | 'sendable' | 'allChecked' | 'checkedIds' | 'groups' | 'current' | 'changes' | 'loading' |
  'loadError' | 'busy' | 'operations' | 'operation' | 'historyLoading' |
  'toggleAll' | 'toggle'>;
const props = defineProps<{ adapter: Pick<SyncAdapter, 'capabilities' | 'kindLabel'>; panel: SyncSidebarPanel }>();
const emit = defineEmits<{ selectChange: [id: string]; selectOperation: [id: string]; send: [] }>();
const { t } = useI18n();
</script>

<template>
  <section class="sync-panel__left">
    <div
      class="sync-panel__list"
      role="list"
      :aria-label="t('syncChanges.operations')"
    >
      <div
        v-for="entry in panel.operations.value"
        :key="entry.id"
        class="sync-panel__item sync-panel__item--tall"
        :class="{ 'sync-panel__item--on': panel.operation.value?.id === entry.id }"
        role="listitem"
      >
        <button
          class="sync-panel__pick sync-panel__pick--stack"
          type="button"
          :aria-label="t('syncChanges.operation', { id: entry.id })"
          @click="emit('selectOperation', entry.id)"
        >
          <strong>{{ entry.title }}</strong>
          <span class="sync-panel__muted">
            {{ t(`syncChanges.operationKinds.${entry.kind}`) }}
            · {{ entry.device ? `${entry.device} · ` : '' }}{{ new Date(entry.at).toLocaleString() }}
          </span>
        </button>
      </div>
      <BaseEmptyState
        v-if="!panel.operations.value.length && !panel.historyLoading.value"
        :title="t('syncChanges.historyEmpty')"
      />
    </div>
  </section>
</template>
