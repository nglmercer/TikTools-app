<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { MetricDescriptor, RankingDefinition } from '@/entities/ranking/types.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseTabs from '@/shared/ui/tabs/BaseTabs.vue';
import { RankingController, parseRankingSnapshot, type RankingSnapshot } from '../../../../widgets/shared/ranking-controller.ts';
import { rankingSummary } from '../lib/metric-labels.ts';
import CompetitionPanel from './CompetitionPanel.vue';
import EntityList from './EntityList.vue';
import RankingDetail from './RankingDetail.vue';
import RankingForm from './RankingForm.vue';

const { t } = useI18n();
const client = useControlClient();
const tab = ref<'rankings' | 'competitions'>('rankings');
const rankings = shallowRef<RankingDefinition[]>([]);
const metrics = shallowRef<MetricDescriptor[]>([]);
const selected = ref('');
/** `null` shows the selected ranking; otherwise the form edits `editing` or creates one. */
const form = shallowRef<{ editing: RankingDefinition | null } | null>(null);
const snapshot = shallowRef<RankingSnapshot | null>(null);
const busy = ref(false);
const error = ref('');
const message = ref('');
const current = computed(() => rankings.value.find((ranking) => ranking.id === selected.value));
const tabs = computed(() => [
  { value: 'rankings' as const, label: t('rankingsPage.tabs.rankings') },
  { value: 'competitions' as const, label: t('rankingsPage.tabs.competitions') },
]);
const listItems = computed(() => rankings.value.map((ranking) => ({ id: ranking.id, title: ranking.id, subtitle: rankingSummary(t, ranking) })));
let disposed = false;
let request = 0;
let snapshotController: RankingController | null = null;

function notify(text: string, failed = false): void {
  error.value = failed ? text : '';
  message.value = failed ? '' : text;
}
async function refresh(): Promise<void> {
  try {
    const [list, descriptors] = await Promise.all([
      client.call<{ rankings: RankingDefinition[] }>('rankings.list', {}),
      client.call<{ metrics: MetricDescriptor[] }>('rankings.metrics', {}),
    ]);
    if (disposed) return;
    rankings.value = list.rankings;
    metrics.value = descriptors.metrics;
    if (selected.value && !rankings.value.some((item) => item.id === selected.value)) selected.value = '';
    if (!selected.value && rankings.value.length) selected.value = rankings.value[0]!.id;
  } catch (failure) { if (!disposed) notify(errorMessage(failure), true); }
}
async function loadSnapshot(): Promise<void> {
  const target = selected.value;
  const token = ++request;
  const initialSnapshot = snapshot.value;
  if (!target) { snapshot.value = null; return; }
  try {
    const result = await client.call<unknown>('rankings.snapshot', { id: target });
    if (!disposed && token === request && target === selected.value) {
      const envelope = { topic: 'ranking.updated', data: { snapshot: result } };
      const next = parseRankingSnapshot(envelope);
      // A push may have arrived while this request was in flight. An older
      // response from another instance cannot be ordered by revision.
      if (next && snapshot.value !== initialSnapshot && snapshot.value?.instanceId !== next.instanceId) return;
      snapshotController?.handleEnvelope(envelope);
    }
  } catch (failure) { if (!disposed && token === request) notify(errorMessage(failure), true); }
}
function select(id: string): void {
  form.value = null; notify('');
  selected.value = id;
}
function openForm(editing: RankingDefinition | null): void {
  notify(''); form.value = { editing };
}
async function saved(id: string, updated: boolean): Promise<void> {
  form.value = null;
  await refresh(); selected.value = id; await loadSnapshot();
  notify(updated ? t('rankingsPage.form.updated') : t('rankingsPage.form.created'));
}
async function operation(method: 'pause' | 'resume' | 'reset'): Promise<void> {
  if (!selected.value) return;
  busy.value = true; notify('');
  try { await client.call(`rankings.${method}`, { id: selected.value }); await loadSnapshot(); }
  catch (failure) { notify(errorMessage(failure), true); } finally { busy.value = false; }
}
watch(selected, (id) => { snapshotController?.dispose(); snapshot.value = null; snapshotController = id ? new RankingController(id, (value) => { snapshot.value = value; }) : null; void loadSnapshot(); });
let offTopic: (() => void) | undefined;
let offGap: (() => void) | undefined;
onMounted(() => {
  offTopic = client.onTopic('ranking.updated', (data) => {
    snapshotController?.handleEnvelope({ topic: 'ranking.updated', data });
  });
  offGap = client.onGap(() => { snapshotController?.clear(); void refresh(); void loadSnapshot(); });
  void refresh();
});
onBeforeUnmount(() => { disposed = true; request += 1; snapshotController?.dispose(); offTopic?.(); offGap?.(); });
</script>

<template>
  <main class="rankings-page">
    <h1>{{ t('rankingsPage.title') }}</h1>
    <p
      v-if="error || message"
      class="notice"
      :class="{ 'notice--error': error }"
      :role="error ? 'alert' : 'status'"
    >
      {{ error || message }}
    </p>
    <BaseTabs
      v-model="tab"
      :tabs="tabs"
    >
      <div
        v-if="tab === 'rankings'"
        class="workspace"
      >
        <EntityList
          :title="t('rankingsPage.list.title')"
          :create-label="t('rankingsPage.list.new')"
          :empty-label="t('rankingsPage.list.emptyTitle')"
          :items="listItems"
          :selected="form ? null : selected"
          @select="select"
          @create="openForm(null)"
        />
        <RankingForm
          v-if="form"
          :key="form.editing?.id ?? 'new'"
          :metrics="metrics"
          :editing="form.editing"
          @saved="saved"
          @cancel="form = null"
        />
        <RankingDetail
          v-else-if="current"
          :ranking="current"
          :snapshot="snapshot"
          :busy="busy"
          @edit="openForm(current)"
          @operation="operation"
          @refresh="loadSnapshot"
          @notify="notify"
        />
      </div>
      <CompetitionPanel
        v-else
        :rankings="rankings"
      />
    </BaseTabs>
  </main>
</template>

<style scoped>
.rankings-page { display: grid; align-content: start; gap: var(--space-3); box-sizing: border-box; width: 100%; max-width: 1400px; padding: var(--space-5); color: var(--color-text); }
h1 { margin: 0; font-size: 22px; }
.notice { margin: 0; padding: var(--space-2) var(--space-3); border-radius: var(--radius-sm); background: var(--color-bg-control); border: 1px solid var(--color-border); }
.notice--error { color: var(--color-danger, #fb7185); border-color: currentColor; }
.workspace { display: grid; grid-template-columns: minmax(220px, 280px) minmax(0, 1fr); gap: var(--space-4); align-items: start; padding-top: var(--space-3); }
@media (max-width: 820px) { .workspace { grid-template-columns: 1fr; } }
</style>
