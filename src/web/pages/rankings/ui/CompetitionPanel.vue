<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue';
import { useI18n } from 'vue-i18n';
import type { CompetitionDefinition } from '@/entities/competition/types.ts';
import type { RankingDefinition } from '@/entities/ranking/types.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import CompetitionDetail from './CompetitionDetail.vue';
import CompetitionForm from './CompetitionForm.vue';
import EntityList from './EntityList.vue';

defineProps<{ rankings: RankingDefinition[] }>();
const { t } = useI18n();
const client = useControlClient();
const items = shallowRef<CompetitionDefinition[]>([]);
const actions = shallowRef<{ id: string; name: string }[]>([]);
const selected = ref('');
/** `null` shows the selected competition; otherwise the form edits `editing` or creates one. */
const form = shallowRef<{ editing: CompetitionDefinition | null } | null>(null);
const error = ref('');
const message = ref('');
const current = computed(() => items.value.find((item) => item.id === selected.value));
const listItems = computed(() => items.value.map((item) => ({ id: item.id, title: item.name, subtitle: `${t(`competitionsPanel.types.${item.kind === 'weighted_poll' ? 'poll' : item.kind}.name`)} · ${item.rankingId}` })));
let disposed = false;

function notify(text: string, failed = false): void {
  error.value = failed ? text : '';
  message.value = failed ? '' : text;
}
async function refresh(): Promise<void> {
  try {
    const response = await client.call<{ competitions: CompetitionDefinition[] }>('competitions.list', {});
    if (disposed) return;
    items.value = response.competitions;
    if (!items.value.some((item) => item.id === selected.value)) selected.value = items.value[0]?.id ?? '';
  } catch (failure) { if (!disposed) notify(errorMessage(failure), true); }
}
async function loadActions(): Promise<void> {
  try {
    const response = await client.call<{ actions: { id: string; name: string; enabled?: boolean }[] }>('automation.list', { kind: 'action' });
    if (!disposed) actions.value = response.actions.filter((action) => action.enabled !== false && typeof action.id === 'string');
  } catch (failure) { if (!disposed) notify(errorMessage(failure), true); }
}
function select(id: string): void {
  form.value = null; notify('');
  selected.value = id;
}
function openForm(editing: CompetitionDefinition | null): void {
  notify(''); form.value = { editing };
}
async function saved(id: string, updated: boolean): Promise<void> {
  form.value = null;
  await refresh(); selected.value = id;
  notify(updated ? t('competitionsPanel.form.updated') : t('competitionsPanel.form.created'));
}
let offGap: (() => void) | undefined;
onMounted(() => {
  offGap = client.onGap(() => { void refresh(); });
  void refresh();
  void loadActions();
});
onBeforeUnmount(() => { disposed = true; offGap?.(); });
</script>

<template>
  <div class="competitions">
    <p
      v-if="error || message"
      class="notice"
      :class="{ 'notice--error': error }"
      :role="error ? 'alert' : 'status'"
    >
      {{ error || message }}
    </p>
    <div class="workspace">
      <EntityList
        :title="t('competitionsPanel.list.title')"
        :create-label="t('competitionsPanel.list.new')"
        :empty-label="t('competitionsPanel.list.empty')"
        :items="listItems"
        :selected="form ? null : selected"
        @select="select"
        @create="openForm(null)"
      />
      <CompetitionForm
        v-if="form"
        :key="form.editing?.id ?? 'new'"
        :rankings="rankings"
        :actions="actions"
        :editing="form.editing"
        @saved="saved"
        @cancel="form = null"
      />
      <CompetitionDetail
        v-else-if="current"
        :key="current.id"
        :competition="current"
        @edit="openForm(current)"
        @notify="notify"
      />
    </div>
  </div>
</template>

<style scoped>
.competitions { display: grid; gap: var(--space-3); padding-top: var(--space-3); color: var(--color-text); }
.notice { margin: 0; padding: var(--space-2) var(--space-3); border-radius: var(--radius-sm); background: var(--color-bg-control); border: 1px solid var(--color-border); }
.notice--error { color: var(--color-danger, #fb7185); border-color: currentColor; }
.workspace { display: grid; grid-template-columns: minmax(220px, 280px) minmax(0, 1fr); gap: var(--space-4); align-items: start; }
@media (max-width: 820px) { .workspace { grid-template-columns: 1fr; } }
</style>
