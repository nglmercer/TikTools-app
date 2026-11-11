<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import {
  buildFieldChoices,
  mergeChoices,
  ROOT_FIELD,
  type FieldSide,
} from '@/entities/cloud-sync/lib/sync-merge.ts';
import type { JsonValue, SyncAdapter, SyncChange, SyncChangeSide } from '@/entities/cloud-sync/api/sync-adapter.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconCloud from '@/shared/ui/icon/icons/IconCloud.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconFork from '@/shared/ui/icon/icons/IconFork.vue';
import IconGitMerge from '@/shared/ui/icon/icons/IconGitMerge.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import IconWarning from '@/shared/ui/icon/icons/IconWarning.vue';
import BaseJsonDiff from '@/shared/ui/json/BaseJsonDiff.vue';
import { diffJson } from '@/shared/ui/json/json-diff.ts';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import { useSyncChanges } from '../model/use-sync-changes.ts';
import { useSyncPanel, type SyncConfirmation, type SyncView } from '../model/use-sync-panel.ts';

const props = defineProps<{ adapter: SyncAdapter }>();
const open = defineModel<boolean>('open', { default: false });
const { t } = useI18n();

const caps = props.adapter.capabilities;
const source = useSyncChanges(props.adapter);
const panel = useSyncPanel(props.adapter, source);
const layout = ref<'unified' | 'side-by-side'>('unified');
const auto = ref(props.adapter.autoSync?.get() ?? false);
const menuOpen = ref(false);

// Each time the panel opens it shows fresh data on the Changes view.
watch(open, (isOpen) => {
  if (!isOpen) return;
  menuOpen.value = false;
  void panel.show('changes');
  void panel.refresh();
});

const title = computed(() => `${t('syncChanges.title')} · ${props.adapter.scope.label}`);
const viewOptions = computed<{ value: SyncView; label: string }[]>(() => [
  { value: 'changes', label: `${t('syncChanges.changes')} ${panel.changes.value.length}` },
  { value: 'history', label: t('syncChanges.history') },
]);
const view = computed({ get: () => panel.view.value, set: (next: SyncView) => { void panel.show(next); } });

const lastSync = computed(() => {
  const at = props.adapter.status?.().lastSyncedAt;
  return at == null ? t('syncChanges.neverSynced') : t('syncChanges.lastSync', { time: new Date(at * 1000).toLocaleString() });
});
const notice = computed(() => props.adapter.notice?.());
const menuEntries = computed(() => [
  ...(caps.force ? [
    { id: 'forcePush', label: t('syncChanges.forcePush'), icon: IconUpload, danger: true },
    { id: 'forcePull', label: t('syncChanges.forcePull'), icon: IconDownload, danger: true },
  ] : []),
  ...(caps.fork ? [{ id: 'fork', label: t('syncChanges.fork'), icon: IconFork, danger: false }] : []),
  ...(caps.reset ? [{ id: 'resetAll', label: t('syncChanges.resetAll'), icon: IconUndo, danger: false }] : []),
]);

/** Escape closes the menu first; the dialog only closes when no menu is open. */
function onMenuEscape(event: KeyboardEvent): void {
  if (!menuOpen.value) return;
  event.preventDefault();
  event.stopPropagation();
  menuOpen.value = false;
}

function onMenuFocusOut(event: FocusEvent): void {
  const next = event.relatedTarget as Node | null;
  if (next && !(event.currentTarget as HTMLElement).contains(next)) menuOpen.value = false;
}

function onMenu(id: string): void {
  menuOpen.value = false;
  if (id === 'fork') void panel.fork();
  else panel.ask({ kind: id as 'forcePush' | 'forcePull' | 'resetAll' });
}

const confirmText = computed(() => {
  const pending = panel.confirmation.value;
  if (!pending) return '';
  const name = 'name' in pending ? pending.name : '';
  return t(`syncChanges.confirm${pending.kind[0]!.toUpperCase()}${pending.kind.slice(1)}`, { name });
});

const sideIcon: Record<SyncChangeSide, typeof IconUpload> = { local: IconUpload, remote: IconDownload, conflict: IconWarning };
/** Local changes are "to send" only when the section can send them. */
const sideKey = computed<Record<SyncChangeSide, string>>(() => ({
  local: caps.push || caps.sync ? 'toSend' : 'localChanges',
  remote: 'toFetch',
  conflict: 'conflicts',
}));

/** The two sides the diff compares, and how they are labelled. */
function sides(change: SyncChange): { before: JsonValue | undefined; after: JsonValue | undefined; beforeLabel: string; afterLabel: string } {
  if (change.side === 'remote') return { before: change.before, after: change.after, beforeLabel: t('syncChanges.local'), afterLabel: t('syncChanges.cloudSide') };
  if (change.side === 'conflict') return { before: change.after, after: change.remote, beforeLabel: t('syncChanges.local'), afterLabel: t('syncChanges.cloudSide') };
  return { before: change.before, after: change.after, beforeLabel: t('syncChanges.synced'), afterLabel: t('syncChanges.current') };
}

const shown = computed(() => (panel.view.value === 'changes' ? panel.current.value : panel.historyChange.value));
const shownSides = computed(() => (shown.value ? sides(shown.value) : null));
const operationStats = computed(() => {
  let added = 0;
  let removed = 0;
  for (const change of panel.operation.value?.changes ?? []) {
    const { stats } = diffJson(change.before, change.after);
    added += stats.added + stats.changed;
    removed += stats.removed + stats.changed;
  }
  return { added, removed };
});

// Field-level conflict choice, rebuilt whenever another element is shown.
const picks = ref<Record<string, FieldSide>>({});
const conflict = computed(() => {
  const change = panel.current.value;
  if (panel.view.value !== 'changes' || !change || change.side !== 'conflict' || !caps.resolve) return null;
  return { change, fields: caps.merge ? buildFieldChoices(change.base, change.after, change.remote) : [] };
});
watch(conflict, (next) => {
  picks.value = Object.fromEntries((next?.fields ?? []).map((field) => [field.key, field.choice]));
}, { immediate: true });

function preview(value: JsonValue | undefined): string {
  if (value === undefined) return '—';
  const text = JSON.stringify(value);
  return text.length > 60 ? `${text.slice(0, 57)}…` : text;
}

function setAll(side: FieldSide): void {
  picks.value = Object.fromEntries((conflict.value?.fields ?? []).map((field) => [field.key, side]));
}

function applyMerge(): void {
  const current = conflict.value;
  if (!current) return;
  const value = mergeChoices(current.change.after, current.change.remote, current.fields, picks.value);
  if (value !== undefined) void panel.resolve(current.change.id, { choice: 'merged', value });
}

const statusLabel = (change: SyncChange): string => t(`syncChanges.status.${change.status}`);
const canReset = computed(() => caps.reset && panel.current.value !== null && panel.current.value.side !== 'remote' && panel.current.value.side !== 'conflict');
const showConfirm = (pending: SyncConfirmation | null): boolean => pending !== null;
</script>

<template>
  <BaseDialog
    v-model:open="open"
    class="sync-panel"
    :title="title"
    :width="1100"
  >
    <template #header-actions>
      <BaseSwitch
        v-if="caps.autoSync && adapter.autoSync"
        v-model="auto"
        class="sync-panel__auto"
        :title="t('syncChanges.autoHint')"
        :disabled="panel.busy.value"
        @update:model-value="adapter.autoSync!.set($event)"
      >
        {{ t('syncChanges.auto') }}
      </BaseSwitch>
      <BaseIconButton
        v-if="caps.fetch"
        :label="t('syncChanges.fetch')"
        variant="default"
        :disabled="panel.busy.value"
        @click="panel.fetchChanges()"
      >
        <IconDownload />
      </BaseIconButton>
      <BaseIconButton
        v-if="caps.push"
        :label="t('syncChanges.push')"
        variant="default"
        :disabled="panel.busy.value || !panel.checkedIds.value.length"
        @click="panel.push()"
      >
        <IconUpload />
      </BaseIconButton>
      <BaseButton
        v-if="caps.sync"
        variant="primary"
        :title="t('syncChanges.syncHint')"
        :disabled="panel.busy.value"
        @click="panel.syncAll()"
      >
        <template #icon>
          <IconRefresh :class="{ 'sync-panel__spin': panel.busy.value }" />
        </template>
        {{ t('syncChanges.sync') }}
      </BaseButton>
      <span
        v-if="menuEntries.length"
        class="sync-panel__more"
        @keydown.escape="onMenuEscape"
        @focusout="onMenuFocusOut"
      >
        <BaseIconButton
          :label="t('syncChanges.more')"
          variant="default"
          :pressed="menuOpen"
          aria-haspopup="menu"
          :aria-expanded="menuOpen"
          :disabled="panel.busy.value"
          @click="menuOpen = !menuOpen"
        >
          <IconMoreVertical />
        </BaseIconButton>
        <ul
          v-if="menuOpen"
          class="sync-panel__menu"
          role="menu"
        >
          <li
            v-for="entry in menuEntries"
            :key="entry.id"
            role="none"
          >
            <button
              class="sync-panel__menu-item"
              :class="{ 'sync-panel__menu-item--danger': entry.danger }"
              type="button"
              role="menuitem"
              @click="onMenu(entry.id)"
            >
              <component :is="entry.icon" />
              {{ entry.label }}
            </button>
          </li>
        </ul>
      </span>
    </template>

    <div class="sync-panel__body">
      <div
        v-if="showConfirm(panel.confirmation.value)"
        class="sync-panel__confirm"
        role="alertdialog"
        :aria-label="t('syncChanges.confirmDialog')"
      >
        <IconWarning />
        <span class="sync-panel__confirm-text">{{ confirmText }}</span>
        <BaseButton @click="panel.cancel()">
          {{ t('common.cancel') }}
        </BaseButton>
        <BaseButton
          variant="danger"
          @click="panel.confirm()"
        >
          {{ t('syncChanges.confirm') }}
        </BaseButton>
      </div>

      <div class="sync-panel__status">
        <span class="sync-panel__cloud">
          <IconCloud />
          <strong>{{ t('syncChanges.cloud') }}</strong>
          <span class="sync-panel__muted">{{ notice ?? lastSync }}</span>
        </span>
        <span class="sync-panel__spacer" />
        <span
          v-for="side in (['local', 'remote', 'conflict'] as const)"
          v-show="panel.counts.value[side] > 0"
          :key="side"
          class="sync-panel__chip"
          :class="{ 'sync-panel__chip--warn': side === 'conflict' }"
          :title="t(`syncChanges.${sideKey[side]}`)"
          :data-side="side"
        >
          <component
            :is="sideIcon[side]"
            :size="12"
          />
          {{ panel.counts.value[side] }}
        </span>
      </div>

      <BaseAlert
        v-if="panel.failure.value || panel.loadError.value"
        tone="error"
        compact
        :title="panel.failure.value ?? t('syncChanges.loadFailed')"
      />

      <div class="sync-panel__main">
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
                    @click="panel.currentId.value = change.id"
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
                @click="panel.push()"
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
                @click="panel.operationId.value = entry.id; panel.historyChangeId.value = null"
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
                  @click="panel.historyChangeId.value = change.id"
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
              <span class="sync-panel__muted">{{ caps.merge ? t('syncChanges.conflictHelp') : t('syncChanges.conflictBoth') }}</span>
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
                  v-if="caps.merge"
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

          <BaseJsonDiff
            v-if="shown && shownSides"
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
      </div>
    </div>
  </BaseDialog>
</template>

<style scoped>
.sync-panel {
  height: min(780px, calc(100vh - var(--space-7) * 2));
}

.sync-panel :deep(.base-dialog__body) {
  padding: 0;
  overflow: hidden;
}

.sync-panel__body {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.sync-panel__auto {
  margin-right: var(--space-3);
}

.sync-panel__more {
  position: relative;
}

.sync-panel__menu {
  position: absolute;
  top: calc(100% + var(--space-2));
  right: 0;
  z-index: var(--z-dropdown);
  min-width: 320px;
  margin: 0;
  padding: var(--space-2);
  border: var(--border-width) solid var(--color-border-strong);
  border-radius: var(--radius-md);
  background: var(--color-bg-overlay);
  box-shadow: var(--shadow-popup);
  list-style: none;
}

.sync-panel__menu-item {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  width: 100%;
  height: var(--control-height-lg);
  padding: 0 var(--space-4);
  border: 0;
  background: transparent;
  color: var(--color-text);
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.sync-panel__menu-item:hover {
  background: var(--color-bg-hover);
}

.sync-panel__menu-item--danger {
  color: var(--color-text-danger);
}

.sync-panel__confirm {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-5);
  border-bottom: var(--border-width) solid var(--color-danger);
  background: color-mix(in srgb, var(--color-danger) 18%, var(--color-bg-panel));
  color: var(--color-text-danger);
}

.sync-panel__confirm-text {
  flex: 1;
  min-width: 200px;
}

.sync-panel__status {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-5);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
}

.sync-panel__cloud {
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
}

.sync-panel__spacer {
  flex: 1;
}

.sync-panel__muted {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.sync-panel__mono {
  font-family: var(--font-family-mono, monospace);
}

.sync-panel__chip {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  height: var(--control-height-sm);
  padding: 0 var(--space-3);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  font-size: var(--font-size-sm);
}

.sync-panel__chip--warn {
  border-color: var(--color-warning);
  color: var(--color-warning);
}

.sync-panel__chip--end {
  margin-left: auto;
}

.sync-panel__main {
  display: flex;
  flex: 1;
  min-height: 0;
}

.sync-panel__left {
  display: flex;
  flex: 0 0 380px;
  flex-direction: column;
  min-width: 0;
  border-right: var(--border-width) solid var(--color-border-subtle);
}

.sync-panel__right {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
}

.sync-panel__switch,
.sync-panel__all,
.sync-panel__send {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-3) var(--space-5);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
}

.sync-panel__send {
  border-top: var(--border-width) solid var(--color-border-subtle);
  border-bottom: 0;
}

.sync-panel__list {
  flex: 1;
  min-height: 0;
  overflow: auto;
}

.sync-panel__group {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-height: 30px;
  padding: 0 var(--space-5);
  border-top: var(--border-width) solid var(--color-border-subtle);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel-header);
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}

.sync-panel__group--warn {
  color: var(--color-warning);
}

.sync-panel__item {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-height: 38px;
  padding: 0 var(--space-5);
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.sync-panel__item--tall {
  min-height: 52px;
}

.sync-panel__item--short {
  min-height: 32px;
}

.sync-panel__item:hover {
  background: var(--color-bg-hover);
}

.sync-panel__item--on,
.sync-panel__item--on:hover {
  background: color-mix(in srgb, var(--color-accent) 35%, var(--color-bg-panel));
}

.sync-panel__pick {
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--space-4);
  min-width: 0;
  min-height: inherit;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--color-text);
  font: inherit;
  text-align: left;
  cursor: pointer;
  transform: none;
}

.sync-panel__pick--stack {
  flex-direction: column;
  align-items: flex-start;
  justify-content: center;
  gap: var(--space-1);
}

.sync-panel__pick:focus-visible {
  outline: var(--border-width) solid var(--color-border-focus);
  outline-offset: -2px;
}

.sync-panel__kind {
  flex: none;
  width: 64px;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.sync-panel__name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sync-panel__state {
  flex: none;
  font-size: var(--font-size-sm);
}

.sync-panel__state--modified { color: var(--color-warning); }
.sync-panel__state--added { color: var(--color-success); }
.sync-panel__state--deleted { color: var(--color-text-danger); }
.sync-panel__state--conflict { color: var(--color-warning); font-weight: var(--font-weight-semibold); }

.sync-panel__head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
  min-height: 46px;
  padding: var(--space-3) var(--space-5);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
}

.sync-panel__title {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sync-panel__add { color: var(--color-success); font-family: var(--font-family-mono, monospace); }
.sync-panel__del { color: var(--color-text-danger); font-family: var(--font-family-mono, monospace); }

.sync-panel__files {
  max-height: 150px;
  overflow: auto;
  border-bottom: var(--border-width) solid var(--color-border-subtle);
}

.sync-panel__conflict {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-4) var(--space-5);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
}

.sync-panel__field {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
}

.sync-panel__key {
  flex: none;
  width: 100px;
  color: var(--color-json-key);
}

.sync-panel__choices {
  display: flex;
  flex: 1;
  gap: var(--space-1);
  min-width: 260px;
  padding: var(--space-1);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-control-active);
}

.sync-panel__choice {
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
  height: var(--control-height-md);
  padding: 0 var(--space-3);
  border: var(--border-width) solid transparent;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--color-text);
  font: inherit;
  cursor: pointer;
  transform: none;
}

.sync-panel__choice:hover {
  background: var(--color-bg-hover);
}

.sync-panel__choice[aria-checked='true'] {
  border-color: var(--color-border-selected);
  background: var(--color-bg-selected);
}

.sync-panel__preview {
  min-width: 0;
  overflow: hidden;
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sync-panel__actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}

.sync-panel__diff {
  flex: 1;
  min-height: 120px;
  margin: var(--space-4) var(--space-5);
}

.sync-panel__spin {
  animation: sync-panel-spin 1s linear infinite;
}

@keyframes sync-panel-spin {
  to { transform: rotate(360deg); }
}

@media (width < 800px) {
  .sync-panel__main {
    flex-direction: column;
    overflow: auto;
  }

  .sync-panel__left {
    flex: none;
    max-height: 45%;
    border-right: 0;
    border-bottom: var(--border-width) solid var(--color-border-subtle);
  }
}
</style>
