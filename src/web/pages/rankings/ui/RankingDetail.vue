<script setup lang="ts">
import { computed, ref } from 'vue';
import { useI18n } from 'vue-i18n';
import type { RankingDefinition } from '@/entities/ranking/types.ts';
import { errorMessage } from '@/shared/api/control-client.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseCheckbox from '@/shared/ui/checkbox/BaseCheckbox.vue';
import BasePanel from '@/shared/ui/panel/BasePanel.vue';
import type { RankingSnapshot } from '../../../../widgets/shared/ranking-controller.ts';
import { rankingSummary } from '../lib/metric-labels.ts';
import FormField from './FormField.vue';
import StatusBadge from './StatusBadge.vue';

type Visualization = 'ranking' | 'bar' | 'pie' | 'donut' | 'poll' | 'roulette';

const props = defineProps<{ ranking: RankingDefinition; snapshot: RankingSnapshot | null; busy: boolean }>();
const emit = defineEmits<{ edit: []; operation: [method: 'pause' | 'resume' | 'reset']; refresh: []; notify: [message: string, failed: boolean] }>();
const { t } = useI18n();
const client = useControlClient();
const visualizations: Visualization[] = ['ranking', 'bar', 'pie', 'donut', 'poll', 'roulette'];
const visualization = ref<Visualization>('ranking');
const showLabels = ref(true);
const showPercentages = ref(false);
const showLegend = ref(true);
const maximum = computed(() => Math.max(1, ...(props.snapshot?.entries.map((entry) => entry.score) ?? [])));
const degraded = computed(() => props.snapshot?.integrityStatus === 'degraded');
const status = computed(() => {
  if (!props.snapshot) return null;
  if (props.snapshot.paused) return { label: t('rankingsPage.status.paused'), tone: 'muted' as const };
  return degraded.value ? { label: t('rankingsPage.status.degraded'), tone: 'warn' as const } : { label: t('rankingsPage.status.running'), tone: 'ok' as const };
});

async function copyUrl(): Promise<void> {
  try {
    const result = await client.call<{ ok: boolean }>('widgets.copyObsUrl', { widget: 'ranking', rankingId: props.ranking.id, visualization: visualization.value, showLabels: showLabels.value, showPercentages: showPercentages.value, showLegend: showLegend.value });
    if (!result.ok) throw new Error(t('rankingsPage.detail.copyFailed'));
    emit('notify', t('rankingsPage.detail.copied'), false);
  } catch (failure) { emit('notify', errorMessage(failure), true); }
}
</script>

<template>
  <BasePanel
    :title="ranking.id"
    :subtitle="rankingSummary(t, ranking)"
  >
    <template #actions>
      <StatusBadge
        v-if="status"
        :label="status.label"
        :tone="status.tone"
      />
    </template>
    <div class="ranking-detail">
      <template v-if="snapshot">
        <dl class="stats">
          <div><dt>{{ t('rankingsPage.detail.participants') }}</dt><dd>{{ snapshot.totalParticipants.toLocaleString() }}</dd></div>
          <div><dt>{{ t('rankingsPage.detail.total') }}</dt><dd>{{ snapshot.totalScore.toLocaleString() }}</dd></div>
        </dl>
        <p
          v-if="degraded"
          class="muted"
        >
          {{ t('rankingsPage.detail.issues', { rejected: snapshot.rejected, dropped: snapshot.dropped, limited: snapshot.limited, cooldown: snapshot.cooldownRejected }) }}
        </p>
        <ol
          v-if="snapshot.entries.length"
          class="leaderboard"
        >
          <li
            v-for="entry in snapshot.entries"
            :key="entry.key"
          >
            <span class="leaderboard__rank">{{ entry.rank }}</span>
            <span class="leaderboard__name">{{ entry.label }} — {{ entry.score }}</span>
            <span
              class="leaderboard__bar"
              :style="{ width: `${(entry.score / maximum) * 100}%` }"
            />
          </li>
        </ol>
        <p
          v-else
          class="muted"
        >
          {{ t('rankingsPage.detail.noEntries') }}
        </p>
      </template>
      <div class="actions">
        <BaseButton
          :disabled="busy"
          @click="emit('operation', snapshot?.paused ? 'resume' : 'pause')"
        >
          {{ snapshot?.paused ? t('rankingsPage.detail.resume') : t('rankingsPage.detail.pause') }}
        </BaseButton>
        <BaseButton
          :disabled="busy || !snapshot?.paused"
          :title="snapshot?.paused ? undefined : t('rankingsPage.detail.editHint')"
          @click="emit('edit')"
        >
          {{ t('rankingsPage.detail.edit') }}
        </BaseButton>
        <BaseButton
          :disabled="busy"
          @click="emit('operation', 'reset')"
        >
          {{ t('rankingsPage.detail.reset') }}
        </BaseButton>
        <BaseButton
          :disabled="busy"
          @click="emit('refresh')"
        >
          {{ t('rankingsPage.detail.refresh') }}
        </BaseButton>
      </div>
      <section class="obs">
        <h3>{{ t('rankingsPage.detail.obsTitle') }}</h3>
        <div class="obs__row">
          <FormField :label="t('rankingsPage.detail.visualization')">
            <select v-model="visualization">
              <option
                v-for="value in visualizations"
                :key="value"
                :value="value"
              >
                {{ t(`rankingsPage.detail.visualizations.${value}`) }}
              </option>
            </select>
          </FormField>
          <template v-if="visualization !== 'roulette'">
            <BaseCheckbox v-model="showLabels">
              {{ t('rankingsPage.detail.showLabels') }}
            </BaseCheckbox>
            <BaseCheckbox v-model="showPercentages">
              {{ t('rankingsPage.detail.showPercentages') }}
            </BaseCheckbox>
            <BaseCheckbox
              v-if="visualization === 'pie' || visualization === 'donut'"
              v-model="showLegend"
            >
              {{ t('rankingsPage.detail.showLegend') }}
            </BaseCheckbox>
          </template>
          <BaseButton
            variant="primary"
            :disabled="busy"
            @click="copyUrl"
          >
            {{ t('rankingsPage.detail.copyUrl') }}
          </BaseButton>
        </div>
      </section>
    </div>
  </BasePanel>
</template>

<style scoped>
.ranking-detail { display: grid; gap: var(--space-4); }
.muted { margin: 0; color: var(--color-text-muted); }
.stats { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: var(--space-2); margin: 0; }
.stats div { padding: var(--space-2) var(--space-3); border-radius: var(--radius-sm); background: var(--color-bg-control); }
.stats dt { font-size: 12px; color: var(--color-text-muted); }
.stats dd { margin: 0; font-size: 20px; font-weight: 700; }
.leaderboard { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; }
.leaderboard li { position: relative; display: flex; gap: var(--space-3); padding: 6px var(--space-2); overflow: hidden; }
.leaderboard__rank { min-width: 22px; font-weight: 700; color: var(--color-text-muted); }
.leaderboard__name { flex: 1; }
.leaderboard__bar { position: absolute; left: 0; bottom: 0; height: 3px; background: var(--color-accent, #3b82f6); opacity: .7; }
.actions { display: flex; flex-wrap: wrap; gap: var(--space-2); }
.obs { display: grid; gap: var(--space-2); padding-top: var(--space-3); border-top: 1px solid var(--color-border); }
.obs h3 { margin: 0; font-size: 14px; }
.obs__row { display: flex; flex-wrap: wrap; align-items: end; gap: var(--space-3); }
.obs__row > :first-child { flex: 0 1 200px; }
</style>
