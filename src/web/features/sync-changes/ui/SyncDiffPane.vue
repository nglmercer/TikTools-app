<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SyncAdapter } from '@/entities/cloud-sync/api/sync-adapter.ts';
import type { SyncPanelState } from '../model/use-sync-panel.ts';
import type { SyncDiffState } from '../model/use-sync-diff.ts';
import { useSyncLabels } from '../lib/use-sync-labels.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconGitMerge from '@/shared/ui/icon/icons/IconGitMerge.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import BaseJsonDiff from '@/shared/ui/json/BaseJsonDiff.vue';

type SyncDetailPanel = Pick<SyncPanelState, 'view' | 'operation' | 'historyChange' | 'current' | 'busy' | 'ask' | 'resolve'>;
const props = defineProps<{ adapter: Pick<SyncAdapter, 'capabilities' | 'kindLabel' | 'restore' | 'resolveLabels'>; panel: SyncDetailPanel; diff: SyncDiffState }>();
const emit = defineEmits<{ selectHistoryChange: [id: string] }>();
const { t } = useI18n();
const caps = props.adapter.capabilities;
const { statusLabel } = useSyncLabels(props.adapter);
const { layout, shown, shownSides, operationStats } = props.diff;
</script>

<template>
  <section class="sync-panel__right">
    <template v-if="panel.view.value === 'history' && panel.operation.value">
      <div class="sync-panel__head">
        <strong class="sync-panel__title">{{ panel.operation.value.title }}</strong>
        <span class="sync-panel__muted sync-panel__mono">{{ panel.operation.value.id }}</span>
        <span class="sync-panel__add">+{{ operationStats.added }}</span>
        <span class="sync-panel__del">−{{ operationStats.removed }}</span>
        <BaseIconButton
          v-if="caps.history && adapter.restore"
          :label="t('syncChanges.revert')"
          :hint="t('syncChanges.revertHint')"
          variant="default"
          :disabled="panel.busy.value"
          @click="panel.ask({ kind: 'revert', operationId: panel.operation.value!.id })"
        >
          <IconUndo />
        </BaseIconButton>
      </div>
      <div
        class="sync-panel__files"
        role="list"
        :aria-label="t('syncChanges.elements', { count: panel.operation.value.changes.length })"
      >
        <div
          v-for="change in panel.operation.value.changes"
          :key="change.id"
          class="sync-panel__item sync-panel__item--short"
          :class="{ 'sync-panel__item--on': panel.historyChange.value?.id === change.id }"
          role="listitem"
        >
          <button
            class="sync-panel__pick"
            type="button"
            :aria-label="t('syncChanges.pickElement', { name: change.name })"
            @click="emit('selectHistoryChange', change.id)"
          >
            <span class="sync-panel__kind">{{ adapter.kindLabel(change.kind) }}</span>
            <span class="sync-panel__name">{{ change.name }}</span>
            <span
              class="sync-panel__state"
              :class="`sync-panel__state--${change.status}`"
            >{{ statusLabel(change) }}</span>
          </button>
          <BaseIconButton
            v-if="adapter.restore"
            :label="t('syncChanges.restore')"
            size="sm"
            :disabled="panel.busy.value"
            @click="panel.ask({ kind: 'restore', operationId: panel.operation.value!.id, id: change.id, name: change.name })"
          >
            <IconUndo />
          </BaseIconButton>
        </div>
      </div>
    </template>

    <BaseEmptyState
      v-else-if="!shown"
      :title="t('syncChanges.pick')"
    >
      <template #icon>
        <IconGitMerge :size="28" />
      </template>
    </BaseEmptyState>

    <div
      v-if="shown?.notice"
      class="sync-panel__keys"
      role="status"
    >
      {{ shown.notice }}
    </div>
    <div
      v-if="shown?.valuesHidden"
      class="sync-panel__keys"
      role="region"
      :aria-label="t('syncChanges.diffRegion')"
    >
      {{ t('syncChanges.variableValuesHidden') }}
    </div>
    <div
      v-else-if="shown?.changedKeys"
      class="sync-panel__keys"
      role="region"
      :aria-label="t('syncChanges.diffRegion')"
    >
      <span>{{ shown.changedKeys.length ? t('syncChanges.keysChanged', { keys: shown.changedKeys.join(', ') }) : t('syncChanges.keysNone') }}</span>
      <span class="sync-panel__muted">{{ t('syncChanges.valuesHidden') }}</span>
    </div>
    <BaseJsonDiff
      v-else-if="shown && shownSides"
      v-model:layout="layout"
      class="sync-panel__diff"
      :label="t('syncChanges.diffRegion')"
      :before="shownSides.before"
      :after="shownSides.after"
      :before-label="shownSides.beforeLabel"
      :after-label="shownSides.afterLabel"
      switchable
      sort-keys
    />
  </section>
</template>
