<script setup lang="ts">
import { computed } from 'vue';
import WidgetStage from '../sdk/WidgetStage.vue';
import { textDefaults, useWidgetText } from '../sdk/text.ts';
import type { RankingSnapshot } from '../shared/ranking-controller.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';

const props = withDefaults(defineProps<{ snapshot: RankingSnapshot | null; visualization: 'ranking' | 'bar' | 'pie' | 'donut' | 'poll'; status: GatewayStatus; debug: boolean; showLabels?: boolean; showPercentages?: boolean; showLegend?: boolean }>(), { showLabels: true, showPercentages: false, showLegend: true });
// The first slice follows the editor accent; the rest keep a readable palette.
const colors = ['var(--widget-accent, #fb7185)', '#38bdf8', '#a78bfa', '#fbbf24', '#34d399', '#f472b6', '#a3e635', '#fb923c'];
const text = useWidgetText('ranking');
// A custom title wins; the default adapts to the poll visualization.
const title = computed(() => { const value = text('title', null); return value === textDefaults.ranking.title && props.visualization === 'poll' ? 'Poll' : value; });
const visibleEntries = computed(() => props.snapshot?.entries.slice(0, 100) ?? []);
const displayedTotal = computed(() => visibleEntries.value.reduce((sum, entry) => sum + entry.score, 0));
const total = computed(() => props.snapshot?.totalScore ?? 0);
const remainder = computed(() => Math.max(0, total.value - displayedTotal.value));
const isChart = computed(() => props.visualization === 'pie' || props.visualization === 'donut');
const percentage = (score: number) => total.value ? `${(score / total.value * 100).toFixed(1)}%` : '0.0%';
const gradient = computed(() => {
  if (!total.value || !props.snapshot) return '#374151';
  let at = 0;
  const segments = visibleEntries.value.map((entry, index) => {
    const start = at;
    at += entry.score / total.value * 100;
    return `${colors[index % colors.length]} ${start}% ${at}%`;
  });
  if (remainder.value) segments.push(`#71717a ${at}% 100%`);
  return `conic-gradient(${segments.join(', ')})`;
});
const maximum = computed(() => visibleEntries.value.reduce((max, entry) => Math.max(max, entry.score), 1));
</script>

<template>
  <WidgetStage :status="status" :debug="debug">
    <section class="ranking-card" aria-label="Ranking" role="status">
      <header><strong v-if="title">{{ title }}</strong><span v-if="snapshot?.paused">Paused</span><span v-if="snapshot?.integrityStatus === 'degraded'">Incomplete</span></header>
      <p v-if="!snapshot" class="empty">Waiting for ranking…</p>
      <p v-else-if="!snapshot.entries.length" class="empty">No contributions yet</p>
      <template v-else>
        <div v-if="isChart" class="chart" :class="{ donut: visualization === 'donut' }" :style="{ background: gradient }" aria-hidden="true"><span v-if="visualization === 'donut'">{{ total.toLocaleString() }}</span></div>
        <ol v-if="!isChart || showLegend" class="rows">
          <li v-for="(entry, index) in visibleEntries" :key="entry.key">
            <span class="rank" :style="{ color: colors[index % colors.length] }">{{ entry.rank }}</span>
            <img v-if="entry.avatarUrl || entry.iconUrl" class="avatar" :src="entry.avatarUrl || entry.iconUrl" alt="" referrerpolicy="no-referrer">
            <span v-if="showLabels" class="name">{{ entry.label }}</span><span v-else class="name"></span>
            <span class="score">{{ entry.score.toLocaleString() }}</span><span v-if="showPercentages || visualization === 'poll'" class="percentage">{{ percentage(entry.score) }}</span>
            <span v-if="visualization === 'bar' || visualization === 'poll'" class="bar" :class="{ 'poll-bar': visualization === 'poll' }" :style="{ width: `${visualization === 'poll' ? (total ? entry.score / total * 100 : 0) : entry.score / maximum * 100}%`, background: colors[index % colors.length] }"></span>
          </li>
          <li v-if="isChart && remainder > 0" class="other"><span class="rank">●</span><span class="name">Other</span><span class="score">{{ remainder.toLocaleString() }}</span><span v-if="showPercentages" class="percentage">{{ percentage(remainder) }}</span></li>
        </ol>
      </template>
    </section>
  </WidgetStage>
</template>

<style scoped>
.ranking-card { width: min(92vw, var(--widget-width, 420px)); max-height: 92vh; overflow: hidden; box-sizing: border-box; padding: var(--widget-padding, 18px); border-radius: var(--widget-radius, 16px); color: var(--widget-textColor, #f5f5f7); background: var(--widget-background, #16161d); border: var(--widget-borderWidth, 1px) solid var(--widget-borderColor, #2a2a33); box-shadow: var(--widget-shadow, 0 12px 32px rgb(0 0 0 / 35%)); opacity: var(--widget-opacity, 1); }
header { display:flex; align-items:center; gap:10px; margin-bottom:12px; } header strong { flex:1; font-size:18px; color: var(--widget-accent, inherit); } header span { font-size:11px; color:#fbbf24; }
.empty { color:#a1a1aa; margin:14px 0; } .rows { list-style:none; margin:0; padding:0; display:grid; gap: var(--widget-gap, 8px); } li { position:relative; display:flex; gap:10px; align-items:center; min-height:28px; padding:4px 6px; overflow:hidden; } .rank { font-weight:700; min-width:24px; } .avatar { width:28px; height:28px; border-radius:50%; object-fit:cover; flex:none; } .name { flex:1; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; } .score { font-variant-numeric:tabular-nums; font-weight:700; } .percentage { color:#a1a1aa; font-variant-numeric:tabular-nums; } .other .rank { color:#71717a; } .bar { position:absolute; left:0; bottom:0; height:3px; opacity:.85; } .poll-bar { height: 5px; } .chart { width: min(38vw, 170px); aspect-ratio:1; border-radius:50%; margin: 10px auto 18px; display:grid; place-items:center; } .donut::before { content:''; grid-area:1 / 1; width:62%; height:62%; border-radius:50%; background:var(--widget-background, #16161d); } .donut span { grid-area:1 / 1; z-index:1; font-weight:700; }
</style>
