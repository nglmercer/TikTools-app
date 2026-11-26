<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import type { JsonValue, SyncAdapter, SyncChange } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { ROOT_FIELD } from '@/entities/cloud-sync/lib/sync-merge.ts';
import type { SyncPanelState } from '../model/use-sync-panel.ts';
import type { SyncDiffState } from '../model/use-sync-diff.ts';
import { useSyncLabels } from '../lib/use-sync-labels.ts';
import ChangeReviewWorkspace from '@/features/review-shared/ui/ChangeReviewWorkspace.vue';
import type { ChangeReviewItem, ChangeReviewStatus } from '@/features/review-shared/model/change-review.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconGitMerge from '@/shared/ui/icon/icons/IconGitMerge.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';

/**
 * The changes view of the sync panel: the same change review workspace used by
 * the library and automation reviews. Every change is a card collapsed into a
 * git-like summary; expanding it reveals the semantic/JSON diff inline, the
 * per-element actions and the conflict resolution — no master/detail split.
 */
const props = defineProps<{ adapter: SyncAdapter; panel: SyncPanelState; diff: SyncDiffState }>();
const { t } = useI18n();
const caps = props.adapter.capabilities;
const panel = props.panel;
const diff = props.diff;
const { sideIcon, sideKey } = useSyncLabels(props.adapter);
const { picks, conflict, preview, setAll, applyMerge, canReset } = diff;

/** The two sides the diff compares for one change, mirroring the old detail pane. */
function sidesOf(change: SyncChange): { before: JsonValue | undefined; after: JsonValue | undefined } {
  if (change.side === 'remote') return { before: change.before, after: change.after };
  if (change.side === 'conflict') return { before: change.after, after: change.remote };
  return { before: change.before, after: change.after };
}

function statusOf(change: SyncChange): ChangeReviewStatus {
  // Conflicts keep the modified accent; the warning side icon marks them.
  return change.status === 'conflict' ? 'modified' : change.status;
}

function itemOf(change: SyncChange): ChangeReviewItem {
  const sides = sidesOf(change);
  return {
    id: change.id,
    title: change.name,
    subtitle: [props.adapter.kindLabel(change.kind), change.device].filter(Boolean).join(' · '),
    status: statusOf(change),
    before: sides.before,
    after: sides.after,
  };
}

const items = computed(() => panel.changes.value.map(itemOf));
const byId = computed(() => new Map(panel.changes.value.map((change) => [change.id, change])));

function sendable(change: SyncChange): boolean {
  return !!(caps.push && change.side === 'local' && change.canSend !== false);
}

/** Expanding a conflict card makes it the current element so its field picks are coherent. */
function onToggle(id: string, open: boolean): void {
  if (open) panel.currentId.value = id;
}

function actionsOf(change: SyncChange): { apply: boolean; withdraw: boolean; reset: boolean } {
  return {
    apply: !!(caps.apply && change.canApply !== false && change.side !== 'conflict'),
    withdraw: !!(caps.withdraw && change.canWithdraw !== false),
    reset: !!(canReset.value && panel.current.value?.id === change.id),
  };
}
</script>

<template>
  <div class="sync-review">
    <div
      v-if="caps.push && panel.sendable.value.length"
      class="sync-review__sendbar"
    >
      <label class="sync-review__all">
        <BaseCheckbox
          :model-value="panel.allChecked.value"
          :indeterminate="panel.checkedIds.value.length > 0 && !panel.allChecked.value"
          :aria-label="t('syncChanges.selectAll')"
          @update:model-value="panel.toggleAll()"
        />
        <span>{{ t('syncChanges.selected', { count: panel.checkedIds.value.length }) }}</span>
      </label>
      <BaseIconButton
        class="sync-review__send"
        :label="t('syncChanges.send', { count: panel.checkedIds.value.length })"
        variant="primary"
        :disabled="panel.busy.value || !panel.checkedIds.value.length"
        @click="panel.push()"
      >
        <IconUpload />
      </BaseIconButton>
    </div>

    <ChangeReviewWorkspace
      :items="items"
      class="sync-review__workspace"
      @toggle="onToggle"
    >
      <template #leading="{ item }">
        <template v-if="byId.get(item.id)">
          <BaseCheckbox
            v-if="sendable(byId.get(item.id)!)"
            class="sync-review__check"
            :model-value="panel.checkedIds.value.includes(item.id)"
            :aria-label="t('syncChanges.sendElement', { name: item.title })"
            @update:model-value="panel.toggle(item.id)"
          />
          <component
            :is="sideIcon[byId.get(item.id)!.side]"
            v-tooltip="t(`syncChanges.${sideKey[byId.get(item.id)!.side]}`)"
            class="sync-review__side"
            :class="`sync-review__side--${byId.get(item.id)!.side}`"
            :size="14"
            :aria-label="t(`syncChanges.${sideKey[byId.get(item.id)!.side]}`)"
          />
        </template>
      </template>

      <template #detail="{ item }">
        <template v-if="byId.get(item.id)">
          <div
            v-if="byId.get(item.id)!.notice"
            class="sync-panel__keys"
            role="status"
          >
            {{ byId.get(item.id)!.notice }}
          </div>
          <div
            v-if="byId.get(item.id)!.valuesHidden"
            class="sync-panel__keys"
            role="status"
          >
            {{ t('syncChanges.variableValuesHidden') }}
          </div>
          <div
            v-else-if="byId.get(item.id)!.changedKeys"
            class="sync-panel__keys"
            role="status"
          >
            <span>{{ byId.get(item.id)!.changedKeys!.length ? t('syncChanges.keysChanged', { keys: byId.get(item.id)!.changedKeys!.join(', ') }) : t('syncChanges.keysNone') }}</span>
            <span class="sync-panel__muted">{{ t('syncChanges.valuesHidden') }}</span>
          </div>

          <div
            v-if="actionsOf(byId.get(item.id)!).apply || actionsOf(byId.get(item.id)!).withdraw || actionsOf(byId.get(item.id)!).reset"
            class="sync-review__actions"
          >
            <BaseIconButton
              v-if="actionsOf(byId.get(item.id)!).apply"
              :label="t('syncChanges.applyCloud')"
              :disabled="panel.busy.value"
              @click="panel.ask({ kind: 'apply', id: item.id, name: item.title })"
            >
              <IconDownload />
            </BaseIconButton>
            <BaseIconButton
              v-if="actionsOf(byId.get(item.id)!).withdraw"
              :label="t('syncChanges.withdraw')"
              :disabled="panel.busy.value"
              @click="panel.ask({ kind: 'withdraw', id: item.id, name: item.title })"
            >
              <IconUndo />
            </BaseIconButton>
            <BaseIconButton
              v-if="actionsOf(byId.get(item.id)!).reset"
              :label="t('syncChanges.reset')"
              :disabled="panel.busy.value"
              @click="panel.ask({ kind: 'reset', id: item.id, name: item.title })"
            >
              <IconUndo />
            </BaseIconButton>
          </div>

          <div
            v-if="conflict && conflict.change.id === item.id && (conflict.change.canResolveLocal !== false || conflict.change.canResolveCloud !== false)"
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
                v-if="conflict.change.canResolveLocal !== false"
                :title="adapter.resolveLabels?.local ?? t('syncChanges.allLocalHint')"
                :disabled="panel.busy.value"
                @click="caps.merge ? setAll('local') : panel.resolve(conflict.change.id, { choice: 'local' })"
              >
                {{ adapter.resolveLabels?.local ?? t('syncChanges.allLocal') }}
              </BaseButton>
              <BaseButton
                v-if="conflict.change.canResolveCloud !== false"
                :title="adapter.resolveLabels?.cloud ?? t('syncChanges.allCloudHint')"
                :disabled="panel.busy.value"
                @click="caps.merge ? setAll('cloud') : panel.resolve(conflict.change.id, { choice: 'cloud' })"
              >
                {{ adapter.resolveLabels?.cloud ?? t('syncChanges.allCloud') }}
              </BaseButton>
            </div>
          </div>
        </template>
      </template>
    </ChangeReviewWorkspace>
  </div>
</template>

<style scoped>
.sync-review {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
  overflow-y: auto;
  padding: var(--space-3) var(--space-5) var(--space-4);
  gap: var(--space-2);
}

.sync-review__workspace {
  flex: none;
}

.sync-review__sendbar {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.sync-review__all {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  cursor: pointer;
}

.sync-review__send {
  margin-left: auto;
}

.sync-review__side {
  flex: none;
  color: var(--color-text-muted);
}

.sync-review__side--local {
  color: var(--color-text-accent);
}

.sync-review__side--remote {
  color: var(--color-text-success);
}

.sync-review__side--conflict {
  color: var(--color-text-warning);
}

.sync-review__check {
  flex: none;
}

.sync-review__actions {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  padding-bottom: var(--space-2);
}
</style>
