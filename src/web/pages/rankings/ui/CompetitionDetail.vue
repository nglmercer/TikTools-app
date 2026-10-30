<script setup lang="ts">
import { computed, inject, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue';
import { useI18n } from 'vue-i18n';
import { routerKey } from 'vue-router';
import type { CompetitionAction, CompetitionDefinition, CompetitionResult, CompetitionState } from '@/entities/competition/types.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import FormField from './FormField.vue';
import StatusBadge from './StatusBadge.vue';

type Operation = 'start' | 'pause' | 'resume' | 'finish' | 'cancel';
type Candidate = { key: string; label: string; score: number };

const props = defineProps<{ competition: CompetitionDefinition }>();
const emit = defineEmits<{ edit: []; notify: [message: string, failed: boolean] }>();
const { t } = useI18n();
const client = useControlClient();
const state = shallowRef<CompetitionState | null>(null);
const results = shallowRef<CompetitionResult[]>([]);
const action = shallowRef<CompetitionAction | null>(null);
const candidates = shallowRef<Candidate[]>([]);
const chosenWinner = ref('');
const busy = ref(false);
/** Buttons offered per lifecycle; finish also retrieves an already stored result. */
const operations: Record<CompetitionState['lifecycle'], Operation[]> = {
  draft: ['start', 'cancel'], active: ['pause', 'finish', 'cancel'], paused: ['resume', 'finish', 'cancel'],
  finalizing: ['finish'], finished: ['finish'], cancelled: [], failed: [],
};
const tones: Partial<Record<CompetitionState['lifecycle'], 'ok' | 'warn' | 'danger'>> = { active: 'ok', paused: 'warn', finalizing: 'warn', cancelled: 'danger', failed: 'danger' };
const id = computed(() => props.competition.id);
let request = 0;
let disposed = false;

async function load(): Promise<void> {
  const target = id.value;
  const token = ++request;
  try {
    const [next, history] = await Promise.all([
      client.call<CompetitionState>('competitions.state', { id: target }),
      client.call<{ results: CompetitionResult[] }>('competitions.results', { id: target }),
    ]);
    const nextAction = next.generation ? await client.call<{ action: CompetitionAction | null }>('competitions.action', { id: target, generation: next.generation }) : { action: null };
    let tied: Candidate[] = [];
    if (next.lifecycle === 'finalizing' && props.competition.selectionMode === 'highest_score' && props.competition.tiePolicy === 'manual') {
      try {
        const response = await client.call<{ generation: number; candidates: Candidate[] }>('competitions.candidates', { id: target });
        if (response.generation === next.generation) tied = response.candidates;
      } catch (failure) { emit('notify', errorMessage(failure), true); }
    }
    if (disposed || token !== request) return;
    state.value = next; results.value = history.results; action.value = nextAction.action; candidates.value = tied;
    if (!tied.some((candidate) => candidate.key === chosenWinner.value)) chosenWinner.value = '';
  } catch (failure) { if (!disposed && token === request) emit('notify', errorMessage(failure), true); }
}
async function run(call: () => Promise<unknown>): Promise<void> {
  busy.value = true; emit('notify', '', false);
  try { await call(); } catch (failure) { emit('notify', errorMessage(failure), true); } finally { busy.value = false; await load(); }
}
const operate = (method: Operation) => run(() => client.call(`competitions.${method}`, { id: id.value }));
const selectWinner = () => run(() => client.call('competitions.selectWinner', { id: id.value, generation: state.value!.generation, winnerId: chosenWinner.value }));
function recoverAction(resolution: 'uncertain' | 'completed' | 'failed'): Promise<void> {
  const generation = action.value!.generation;
  return run(() => resolution === 'uncertain'
    ? client.call('competitions.markActionUncertain', { id: id.value, generation })
    : client.call('competitions.resolveAction', { id: id.value, generation, resolution }));
}
async function copyRevealUrl(): Promise<void> {
  try {
    const result = await client.call<{ ok: boolean }>('widgets.copyObsUrl', { widget: 'ranking', rankingId: props.competition.rankingId, visualization: 'roulette', competitionId: id.value });
    if (!result.ok) throw new Error(t('competitionsPanel.operate.copyFailed'));
    emit('notify', t('competitionsPanel.operate.copied'), false);
  } catch (failure) { emit('notify', errorMessage(failure), true); }
}

// Opens Automations with a new rule triggered by this event type.
const router = inject(routerKey, null);
function automateWinner(): void {
  void router?.push({ name: 'automations', query: { trigger: 'competition.winner.selected' } });
}

const subscriptions: (() => void)[] = [];
onMounted(() => {
  for (const topic of ['competition.started', 'competition.paused', 'competition.resumed', 'competition.finished', 'competition.cancelled', 'competition.action.completed', 'competition.action.failed']) {
    subscriptions.push(client.onTopic<{ competitionId?: string }>(topic, (data) => { if (data?.competitionId === id.value) void load(); }));
  }
  subscriptions.push(client.onGap(() => { void load(); }));
  void load();
});
onBeforeUnmount(() => { disposed = true; subscriptions.forEach((off) => off()); });
</script>

<template>
  <BasePanel
    :title="competition.name"
    :subtitle="t(`competitionsPanel.types.${competition.kind === 'weighted_poll' ? 'poll' : competition.kind}.name`)"
  >
    <template #actions>
      <StatusBadge
        v-if="state"
        :label="t(`competitionsPanel.lifecycle.${state.lifecycle}`)"
        :tone="tones[state.lifecycle]"
      />
    </template>
    <div
      v-if="state"
      class="competition-detail"
    >
      <p
        v-if="state.deadlineMs || state.remainingMs !== null"
        class="muted"
      >
        {{ state.deadlineMs ? t('competitionsPanel.operate.ends', { time: new Date(state.deadlineMs).toLocaleString() }) : t('competitionsPanel.operate.remaining', { seconds: Math.ceil(state.remainingMs! / 1000) }) }}
      </p>
      <div class="actions">
        <BaseButton
          v-for="operation in operations[state.lifecycle]"
          :key="operation"
          :variant="operation === 'start' ? 'primary' : 'default'"
          :disabled="busy"
          @click="operate(operation)"
        >
          {{ t(`competitionsPanel.operate.${operation}`) }}
        </BaseButton>
        <BaseButton
          v-if="state.lifecycle === 'draft'"
          :disabled="busy"
          @click="emit('edit')"
        >
          {{ t('competitionsPanel.operate.edit') }}
        </BaseButton>
        <BaseButton @click="load">
          {{ t('competitionsPanel.operate.refresh') }}
        </BaseButton>
      </div>
      <div
        v-if="candidates.length"
        class="actions actions--end"
      >
        <FormField :label="t('competitionsPanel.operate.tiedWinner')">
          <select v-model="chosenWinner">
            <option value="">
              {{ t('competitionsPanel.operate.chooseTied') }}
            </option>
            <option
              v-for="candidate in candidates"
              :key="candidate.key"
              :value="candidate.key"
            >
              {{ candidate.label || candidate.key }} — {{ candidate.score }}
            </option>
          </select>
        </FormField>
        <BaseButton
          variant="primary"
          :disabled="busy || !chosenWinner"
          @click="selectWinner"
        >
          {{ t('competitionsPanel.operate.selectWinner') }}
        </BaseButton>
      </div>
      <section
        v-if="results.length || action"
        class="block"
      >
        <h3>{{ t('competitionsPanel.operate.history') }}</h3>
        <ul>
          <li
            v-for="result in results"
            :key="result.generation"
          >
            {{ t('competitionsPanel.operate.result', { n: result.generation, winner: result.selection.winner ?? t('competitionsPanel.operate.noWinner') }) }}
          </li>
        </ul>
        <div
          v-if="action"
          class="actions"
        >
          <span>{{ t('competitionsPanel.operate.action', { id: action.actionId, status: action.status }) }}<template v-if="action.reason"> ({{ action.reason }})</template></span>
          <BaseButton
            v-if="action.status === 'running'"
            :disabled="busy"
            @click="recoverAction('uncertain')"
          >
            {{ t('competitionsPanel.operate.markUncertain') }}
          </BaseButton>
          <template v-if="action.status === 'uncertain'">
            <BaseButton
              :disabled="busy"
              @click="recoverAction('completed')"
            >
              {{ t('competitionsPanel.operate.markCompleted') }}
            </BaseButton>
            <BaseButton
              :disabled="busy"
              @click="recoverAction('failed')"
            >
              {{ t('competitionsPanel.operate.markFailed') }}
            </BaseButton>
          </template>
        </div>
      </section>
      <section class="block block--row">
        <BaseButton @click="copyRevealUrl">
          {{ t('competitionsPanel.operate.copyReveal') }}
        </BaseButton>
        <BaseButton
          v-if="router"
          @click="automateWinner"
        >
          {{ t('competitionsPanel.operate.automate') }}
        </BaseButton>
      </section>
    </div>
  </BasePanel>
</template>

<style scoped>
.competition-detail { display: grid; gap: var(--space-3); }
.muted { margin: 0; color: var(--color-text-muted); }
.actions { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-2); }
.actions--end { align-items: end; }
.block { display: grid; gap: var(--space-2); padding-top: var(--space-3); border-top: 1px solid var(--color-border); justify-items: start; }
.block--row { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-3); }
.block h3 { margin: 0; font-size: 14px; }
.block ul { margin: 0; padding-left: var(--space-4); }
</style>
