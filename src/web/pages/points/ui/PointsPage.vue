<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useConnectionStore } from '@/entities/connection/model/connection-store.ts';
import { isConnectionBusy } from '@/entities/connection/lib/connection-status.ts';
import { debounce, loadViewState, saveViewState, type PointsViewState } from '@/shared/lib/view-state.ts';
import PointsRulesPanel from '@/features/configure-points/ui/PointsRulesPanel.vue';
import type { TableSort } from '@/shared/ui/table/table-types.ts';
import PointsLeaderboardPanel from '@/widgets/points-leaderboard/ui/PointsLeaderboardPanel.vue';

const sortKeys: Record<NonNullable<PointsViewState['sortBy']>, string> = {
  points: 'points',
  level: 'level',
  viewer: 'uniqueId',
};

function savedSort(state: PointsViewState | undefined): TableSort | null {
  if (!state?.sortBy) return null;
  return { key: sortKeys[state.sortBy], direction: state.sortDir ?? 'desc' };
}

function storedSort(sort: TableSort | null): Pick<PointsViewState, 'sortBy' | 'sortDir'> {
  const entry = Object.entries(sortKeys).find(([, key]) => key === sort?.key);
  return entry && sort ? { sortBy: entry[0] as PointsViewState['sortBy'], sortDir: sort.direction } : {};
}

const connection = useConnectionStore();
const saved = loadViewState().points;
const query = ref(saved?.searchQuery ?? '');
const sort = ref<TableSort | null>(savedSort(saved));

const locked = computed(() => {
  const status = connection.status;
  return status === 'connected' || isConnectionBusy(status);
});

const persist = debounce(() => {
  saveViewState('points', { searchQuery: query.value, ...storedSort(sort.value) });
});
watch([query, sort], persist);
</script>

<template>
  <div
    class="points-page"
    data-guide-scope="points"
  >
    <PointsRulesPanel
      data-guide="points-rules"
      class="points-page__rules"
    />
    <PointsLeaderboardPanel
      v-model:query="query"
      v-model:sort="sort"
      data-guide="points-leaderboard"
      class="points-page__leaderboard"
      :locked="locked"
    />
  </div>
</template>

<style scoped>
.points-page {
  display: flex;
  flex: 1;
  gap: var(--space-4);
  min-width: 0;
  min-height: 0;
  padding: var(--space-4);
}

.points-page__rules {
  flex: none;
  width: 440px;
  overflow: hidden;
}

.points-page__leaderboard {
  flex: 1;
}

@media (width < 1100px) {
  .points-page {
    flex-direction: column;
    overflow-y: auto;
  }

  .points-page__rules {
    width: auto;
  }

  .points-page__leaderboard {
    flex: none;
    min-height: 420px;
  }
}
</style>
