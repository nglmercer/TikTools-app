<script setup lang="ts">
import { computed, onMounted, shallowRef } from 'vue';
import { useI18n } from 'vue-i18n';
import type { CompetitionDefinition } from '@/entities/competition/types.ts';
import type { RankingDefinition } from '@/entities/ranking/types.ts';
import { useControlClient } from '@/shared/api/use-control-client.ts';

/** Options sent with `widgets.copyObsUrl` for the ranking widget. */
export interface RankingSourceSettings {
  rankingId: string;
  visualization: 'ranking' | 'bar' | 'pie' | 'donut' | 'poll' | 'roulette';
  competitionId: string;
  showLabels: boolean;
  showPercentages: boolean;
  showLegend: boolean;
}

const settings = defineModel<RankingSourceSettings>({ required: true });
const { t } = useI18n();
const client = useControlClient();
const rankings = shallowRef<RankingDefinition[]>([]);
const competitions = shallowRef<CompetitionDefinition[]>([]);
const visualizations = ['ranking', 'bar', 'pie', 'donut', 'poll', 'roulette'] as const;
const revealable = computed(() => competitions.value.filter((item) => item.rankingId === settings.value.rankingId));

function update(patch: Partial<RankingSourceSettings>): void {
  settings.value = { ...settings.value, ...patch };
}

onMounted(async () => {
  const [list, contests] = await Promise.allSettled([
    client.call<{ rankings: RankingDefinition[] }>('rankings.list', {}),
    client.call<{ competitions: CompetitionDefinition[] }>('competitions.list', {}),
  ]);
  if (list.status === 'fulfilled') rankings.value = list.value.rankings;
  if (contests.status === 'fulfilled') competitions.value = contests.value.competitions;
});
</script>

<template>
  <div class="ranking-source">
    <label>{{ t('overlays.rankingSource.ranking') }}
      <select
        :value="settings.rankingId"
        @change="update({ rankingId: ($event.target as HTMLSelectElement).value, competitionId: '' })"
      >
        <option value="">{{ t('overlays.rankingSource.choose') }}</option>
        <option
          v-for="ranking in rankings"
          :key="ranking.id"
          :value="ranking.id"
        >{{ ranking.id }}</option>
      </select>
    </label>
    <label>{{ t('overlays.rankingSource.visualization') }}
      <select
        :value="settings.visualization"
        @change="update({ visualization: ($event.target as HTMLSelectElement).value as RankingSourceSettings['visualization'] })"
      >
        <option
          v-for="value in visualizations"
          :key="value"
          :value="value"
        >{{ t(`overlays.rankingSource.visualizations.${value}`) }}</option>
      </select>
    </label>
    <label v-if="settings.visualization === 'roulette'">{{ t('overlays.rankingSource.competition') }}
      <select
        :value="settings.competitionId"
        @change="update({ competitionId: ($event.target as HTMLSelectElement).value })"
      >
        <option value="">—</option>
        <option
          v-for="competition in revealable"
          :key="competition.id"
          :value="competition.id"
        >{{ competition.name }}</option>
      </select>
    </label>
    <template v-else>
      <label><input
        type="checkbox"
        :checked="settings.showLabels"
        @change="update({ showLabels: ($event.target as HTMLInputElement).checked })"
      > {{ t('overlays.rankingSource.labels') }}</label>
      <label><input
        type="checkbox"
        :checked="settings.showPercentages"
        @change="update({ showPercentages: ($event.target as HTMLInputElement).checked })"
      > {{ t('overlays.rankingSource.percentages') }}</label>
      <label><input
        type="checkbox"
        :checked="settings.showLegend"
        @change="update({ showLegend: ($event.target as HTMLInputElement).checked })"
      > {{ t('overlays.rankingSource.legend') }}</label>
    </template>
  </div>
</template>

<style scoped>
.ranking-source { display: flex; flex-wrap: wrap; gap: 8px 16px; align-items: center; margin-bottom: 8px; }
.ranking-source label { display: flex; gap: 6px; align-items: center; }
</style>
