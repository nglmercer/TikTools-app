<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { SyncAdapter } from '@/entities/cloud-sync/api/sync-adapter.ts';
import BaseAlert from '@/shared/ui/alert/BaseAlert.vue';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseDialog from '@/shared/ui/dialog/BaseDialog.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconFork from '@/shared/ui/icon/icons/IconFork.vue';
import IconUndo from '@/shared/ui/icon/icons/IconUndo.vue';
import IconCloud from '@/shared/ui/icon/icons/IconCloud.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconUpload from '@/shared/ui/icon/icons/IconUpload.vue';
import IconWarning from '@/shared/ui/icon/icons/IconWarning.vue';
import BaseSwitch from '@/shared/ui/switch/BaseSwitch.vue';
import { useSyncChanges } from '../model/use-sync-changes.ts';
import { useSyncPanel, type SyncConfirmation, type SyncView } from '../model/use-sync-panel.ts';
import SyncSidebar from './SyncSidebar.vue';
import SyncDiffPane from './SyncDiffPane.vue';
import { useSyncLabels } from '../lib/use-sync-labels.ts';
import { useSyncDiff } from '../model/use-sync-diff.ts';

const props = withDefaults(defineProps<{ adapter: SyncAdapter; preparing?: boolean }>(), { preparing: false });
const open = defineModel<boolean>('open', { default: false });
const { t } = useI18n();

const caps = props.adapter.capabilities;
const source = useSyncChanges(props.adapter, computed(() => !props.preparing));
const panel = useSyncPanel(props.adapter, source);
const auto = ref(props.adapter.autoSync?.get() ?? false);
const menuOpen = ref(false);

// Each time the panel opens it shows fresh data on the Changes view.
watch(open, (isOpen) => {
  if (!isOpen || props.preparing) return;
  menuOpen.value = false;
  void panel.show('changes');
  void panel.refresh();
});

const title = computed(() => `${t('syncChanges.title')} · ${props.adapter.scope.label}`);
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

const { sideIcon, sideKey } = useSyncLabels(props.adapter);
const diff = useSyncDiff(props.adapter, panel);
const showConfirm = (pending: SyncConfirmation | null): boolean => pending !== null;
</script>

<template>
  <BaseDialog
    v-model:open="open"
    class="sync-panel"
    :class="{ 'sync-panel--preparing': preparing }"
    :title="title"
    :width="preparing ? 760 : 1100"
  >
    <template #header-actions>
      <template v-if="!preparing">
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

        <BaseIconButton
          v-if="caps.markReviewed && adapter.markReviewed"
          :label="t('syncChanges.markReviewed')"
          :hint="t('syncChanges.markReviewedHint')"
          variant="default"
          :disabled="panel.busy.value || panel.changes.value.length === 0"
          @click="panel.ask({ kind: 'markReviewed' })"
        >
          <IconCheck />
        </BaseIconButton>
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
    </template>

    <section
      v-if="preparing"
      class="sync-panel__preparation"
    >
      <slot name="preparation" />
    </section>
    <div
      v-else
      class="sync-panel__body"
    >
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
          <strong>{{ adapter.requiresCloud === false ? t('syncChanges.thisDevice') : t('syncChanges.cloud') }}</strong>
          <span
            class="sync-panel__muted sync-panel__notice"
            :title="notice"
          >{{ notice ?? lastSync }}</span>
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
        <SyncSidebar
          v-model:view="view"
          :adapter="adapter"
          :panel="panel"
          @send="panel.push()"
          @select-change="panel.currentId.value = $event"
          @select-operation="panel.operationId.value = $event; panel.historyChangeId.value = null"
        />
        <SyncDiffPane
          :adapter="adapter"
          :panel="panel"
          :diff="diff"
          @select-history-change="panel.historyChangeId.value = $event"
        />
      </div>
    </div>
    <template
      v-if="!preparing"
      #footer
    >
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
    </template>
  </BaseDialog>
</template>

<style src="./sync-panel.css"></style>
