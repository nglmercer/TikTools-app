<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { SyncAdapter } from '@/entities/cloud-sync/api/sync-adapter.ts';
import type { SyncPanelState, SyncView } from '../model/use-sync-panel.ts';
import { useSyncLabels } from '../lib/use-sync-labels.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';

type SyncSidebarPanel = Pick<SyncPanelState,
  'view' | 'sendable' | 'allChecked' | 'checkedIds' | 'groups' | 'current' | 'changes' | 'loading' |
  'busy' | 'operations' | 'operation' | 'historyLoading' |
  'toggleAll' | 'toggle'>;
const props = defineProps<{ adapter: Pick<SyncAdapter, 'capabilities' | 'kindLabel'>; panel: SyncSidebarPanel }>();
const emit = defineEmits<{ selectChange: [id: string]; selectOperation: [id: string]; send: [] }>();
const view = defineModel<SyncView>('view', { required: true });
const { t } = useI18n();
const caps = props.adapter.capabilities;
const { sideIcon, sideKey, statusLabel } = useSyncLabels(props.adapter);
const viewOptions = computed(() => [
  { value: 'changes' as const, label: `${t('syncChanges.changes')} ${props.panel.changes.value.length}` },
  { value: 'history' as const, label: t('syncChanges.history') },
]);
</script>

<template>
  <section class="sync-panel__left">
    <div
      v-if="caps.history"
      class="sync-panel__switch"
    >
      <BaseSegmented
        v-model="view"
        :options="viewOptions"
        size="sm"
        :aria-label="t('syncChanges.view')"
      />
    </div>

    <template v-if="panel.view.value === 'changes'">
      <label
        v-if="panel.sendable.value.length"
        class="sync-panel__all"
      >
        <BaseCheckbox
          :model-value="panel.allChecked.value"
          :indeterminate="panel.checkedIds.value.length > 0 && !panel.allChecked.value"
          :aria-label="t('syncChanges.selectAll')"
          @update:model-value="panel.toggleAll()"
        />
        <span>{{ t('syncChanges.selected', { count: panel.checkedIds.value.length }) }}</span>
      </label>
      <div
        class="sync-panel__list"
        role="list"
        :aria-label="t('syncChanges.list')"
      >
        <template
          v-for="group in panel.groups.value"
          :key="group.side"
        >
          <div
            class="sync-panel__group"
            :class="{ 'sync-panel__group--warn': group.side === 'conflict' }"
          >
            <component
              :is="sideIcon[group.side]"
              :size="13"
            />
            {{ t(`syncChanges.${sideKey[group.side]}`) }}
            <span class="sync-panel__chip sync-panel__chip--end">{{ group.changes.length }}</span>
          </div>
          <div
            v-for="change in group.changes"
            :key="change.id"
            class="sync-panel__item"
            :class="{ 'sync-panel__item--on': panel.current.value?.id === change.id }"
            role="listitem"
          >
            <BaseCheckbox
              v-if="caps.push && change.side === 'local'"
              :model-value="panel.checkedIds.value.includes(change.id)"
              :aria-label="t('syncChanges.sendElement', { name: change.name })"
              @update:model-value="panel.toggle(change.id)"
            />
            <button
              class="sync-panel__pick"
              type="button"
              :aria-label="t('syncChanges.pickElement', { name: change.name })"
              :aria-current="panel.current.value?.id === change.id || undefined"
              @click="emit('selectChange', change.id)"
            >
              <span class="sync-panel__kind">{{ adapter.kindLabel(change.kind) }}</span>
              <span class="sync-panel__name">{{ change.name }}</span>
              <span
                v-if="change.device"
                class="sync-panel__muted"
              >{{ change.device }}</span>
              <span
                class="sync-panel__state"
                :class="`sync-panel__state--${change.status}`"
              >{{ statusLabel(change) }}</span>
            </button>
          </div>
        </template>
        <BaseEmptyState
          v-if="!panel.changes.value.length && !panel.loading.value"
          :title="t('syncChanges.empty')"
        >
          <template #icon>
            <IconCheck :size="28" />
          </template>
        </BaseEmptyState>
      </div>
      <div
        v-if="caps.push"
        class="sync-panel__send"
      >
        <BaseButton
          variant="primary"
          block
          :disabled="panel.busy.value || !panel.checkedIds.value.length"
          @click="emit('send')"
        >
          <template #icon>
            <IconUpload />
          </template>
          {{ t('syncChanges.send', { count: panel.checkedIds.value.length }) }}
        </BaseButton>
      </div>
    </template>

    <div
      v-else
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
