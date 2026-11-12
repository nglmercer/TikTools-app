<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import type { SyncAdapter } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { ROOT_FIELD } from '@/entities/cloud-sync/lib/sync-merge.ts';
import type { SyncPanelState } from '../model/use-sync-panel.ts';
import type { SyncDiffState } from '../model/use-sync-diff.ts';
import { useSyncLabels } from '../lib/use-sync-labels.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconGitMerge from '@/shared/ui/icon/icons/IconGitMerge.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import BaseJsonDiff from '@/shared/ui/json/BaseJsonDiff.vue';

type SyncDetailPanel = Pick<SyncPanelState, 'view' | 'operation' | 'historyChange' | 'current' | 'busy' | 'ask' | 'resolve'>;
const props = defineProps<{ adapter: Pick<SyncAdapter, 'capabilities' | 'kindLabel' | 'restore'>; panel: SyncDetailPanel; diff: SyncDiffState }>();
const emit = defineEmits<{ selectHistoryChange: [id: string] }>();
const { t } = useI18n();
const caps = props.adapter.capabilities;
const { statusLabel } = useSyncLabels(props.adapter);
const { layout, shown, shownSides, operationStats, picks, conflict, preview, setAll, applyMerge, canReset } = props.diff;
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

    <template v-else-if="panel.view.value === 'changes' && panel.current.value">
      <div class="sync-panel__head">
        <strong class="sync-panel__title">{{ panel.current.value.name }}</strong>
        <span class="sync-panel__chip">{{ adapter.kindLabel(panel.current.value.kind) }}</span>
        <span
          class="sync-panel__state"
          :class="`sync-panel__state--${panel.current.value.status}`"
        >{{ statusLabel(panel.current.value) }}</span>
        <span class="sync-panel__spacer" />
        <BaseIconButton
          v-if="canReset"
          :label="t('syncChanges.reset')"
          variant="default"
          :disabled="panel.busy.value"
          @click="panel.ask({ kind: 'reset', id: panel.current.value!.id, name: panel.current.value!.name })"
        >
          <IconUndo />
        </BaseIconButton>
      </div>
      <div
        v-if="conflict"
        class="sync-panel__conflict"
      >
        <span class="sync-panel__muted">{{ conflict.merge ? t('syncChanges.conflictHelp') : t('syncChanges.conflictBoth') }}</span>
        <div
          v-for="field in conflict.fields"
          :key="field.key"
          class="sync-panel__field"
        >
          <span class="sync-panel__mono sync-panel__key">{{ field.key === ROOT_FIELD ? t('syncChanges.rootField') : field.key }}</span>
          <div
            class="sync-panel__choices"
            role="radiogroup"
            :aria-label="field.key"
          >
            <button
              v-for="side in (['base', 'local', 'cloud'] as const)"
              :key="side"
              class="sync-panel__choice"
              type="button"
              role="radio"
              :aria-checked="picks[field.key] === side"
              :title="t(`syncChanges.${side === 'cloud' ? 'cloudHint' : `${side}Hint`}`)"
              @click="picks = { ...picks, [field.key]: side }"
            >
              <span class="sync-panel__muted">{{ t(`syncChanges.${side === 'cloud' ? 'cloudSide' : side}`) }}</span>
              <span class="sync-panel__mono sync-panel__preview">{{ preview(field[side]) }}</span>
            </button>
          </div>
        </div>
        <div class="sync-panel__actions">
          <BaseButton
            v-if="conflict.merge"
            variant="primary"
            :disabled="panel.busy.value"
            @click="applyMerge"
          >
            <template #icon>
              <IconGitMerge />
            </template>
            {{ t('syncChanges.applyMerge') }}
          </BaseButton>
          <BaseButton
            :title="t('syncChanges.allLocalHint')"
            :disabled="panel.busy.value"
            @click="caps.merge ? setAll('local') : panel.resolve(conflict.change.id, { choice: 'local' })"
          >
            {{ t('syncChanges.allLocal') }}
          </BaseButton>
          <BaseButton
            :title="t('syncChanges.allCloudHint')"
            :disabled="panel.busy.value"
            @click="caps.merge ? setAll('cloud') : panel.resolve(conflict.change.id, { choice: 'cloud' })"
          >
            {{ t('syncChanges.allCloud') }}
          </BaseButton>
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
      v-if="shown?.changedKeys"
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
