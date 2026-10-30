<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import WidgetStage from '../sdk/WidgetStage.vue';
import type { GatewayStatus } from '../shared/gateway-client.ts';
import type { RevealResult } from '../shared/reveal-controller.ts';
const props = defineProps<{ result: RevealResult | null; status: GatewayStatus; debug: boolean }>();
const revealed = ref(false);
watch(() => props.result, () => { revealed.value = false; }, { immediate: true });
const colors = ['#fb7185', '#38bdf8', '#a78bfa', '#fbbf24', '#34d399', '#f472b6', '#a3e635', '#fb923c'];
const slices = computed(() => {
  const eligible = props.result?.eligible ?? [];
  const weights = eligible.map((entry) => props.result?.mode === 'weighted_random' ? entry.weight : 1);
  const total = weights.reduce((sum, value) => sum + value, props.result?.otherWeight ?? 0);
  if (!total || !Number.isSafeInteger(total)) return null;
  let at = 0; let winnerMidpoint = 0;
  const segments = eligible.map((entry, index) => {
    const start = at; at += weights[index]! / total * 360;
    if (entry.key === props.result?.winner) winnerMidpoint = (start + at) / 2;
    return `${colors[index % colors.length]} ${start}deg ${at}deg`;
  });
  if (props.result?.otherWeight) segments.push(`#71717a ${at}deg 360deg`);
  return { gradient: `conic-gradient(${segments.join(', ')})`, rotation: 1800 - winnerMidpoint };
});
</script>
<template>
  <WidgetStage :status="status" :debug="debug">
    <section class="reveal" role="status" aria-label="Competition result">
      <strong>Competition result</strong>
      <p v-if="!result">Waiting for a finalized result…</p>
      <p v-else-if="!result.winner">No winner</p>
      <template v-else>
        <div class="pointer" aria-hidden="true">▼</div>
        <div v-if="slices" :key="`${result.competitionId}:${result.generation}`" class="wheel" :style="{ background: slices.gradient, '--rotation': `${slices.rotation}deg` }" aria-hidden="true" @animationend="revealed = true"></div>
        <p v-if="revealed || !slices">Winner: <strong>{{ result.winner }}</strong></p>
        <p v-else>Revealing winner…</p>
        <p v-if="result.eligibleCount > result.eligible.length">Other: {{ result.eligibleCount - result.eligible.length }} participants</p>
      </template>
    </section>
  </WidgetStage>
</template>
<style scoped>
.reveal { width:min(92vw, 420px); padding:20px; border-radius:16px; color:var(--widget-textColor,#f5f5f7); background:var(--widget-background,#16161d); text-align:center; }
.pointer { font-size:24px; position:relative; z-index:1; }
.wheel { width:min(70vw,240px); aspect-ratio:1; border-radius:50%; margin:auto; border:4px solid white; animation:reveal 3s cubic-bezier(.16,.7,.12,1) both; }
@keyframes reveal { from { transform:rotate(0deg) } to { transform:rotate(var(--rotation)) } }
@media (prefers-reduced-motion: reduce) { .wheel { animation-duration:.01ms; } }
</style>
