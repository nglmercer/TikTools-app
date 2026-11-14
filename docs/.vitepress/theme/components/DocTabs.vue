<script setup lang="ts">
import { nextTick, ref, useId } from 'vue';
defineProps<{ label: string }>();
const id = useId();
const options = ['windows', 'linux'] as const;
const selected = ref<typeof options[number]>('windows');
const buttons = ref<HTMLButtonElement[]>([]);
async function onKey(event: KeyboardEvent) {
  if (!['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
  event.preventDefault();
  const index = options.indexOf(selected.value);
  const next = event.key === 'Home' ? 0 : event.key === 'End' ? 1 : 1 - index;
  selected.value = options[next]!;
  await nextTick();
  buttons.value[next]?.focus();
}
</script>
<template>
  <section class="doc-tabs" :aria-label="label">
    <div class="doc-tabs__list" role="tablist" :aria-label="label" @keydown="onKey">
      <button v-for="(option, index) in options" :id="`${id}-${option}-tab`" :key="option" :ref="(element) => { if (element) buttons[index] = element as HTMLButtonElement; }" type="button" role="tab" :aria-selected="selected === option" :aria-controls="`${id}-${option}-panel`" :tabindex="selected === option ? 0 : -1" @click="selected = option"><span aria-hidden="true">{{ option === 'windows' ? '⊞' : '>_' }}</span>{{ option === 'windows' ? 'Windows' : 'Linux' }}</button>
    </div>
    <div v-for="option in options" v-show="selected === option" :id="`${id}-${option}-panel`" :key="option" class="doc-tabs__panel" role="tabpanel" :aria-labelledby="`${id}-${option}-tab`" tabindex="0"><slot :name="option" /></div>
  </section>
</template>
<style>
.doc-tabs { margin:24px 0; border:1px solid var(--vp-c-divider); border-radius:10px; overflow:hidden; }
.doc-tabs__list { display:flex; gap:4px; padding:6px; background:var(--vp-c-bg-soft); border-bottom:1px solid var(--vp-c-divider); }
.doc-tabs__list button { display:flex; align-items:center; justify-content:center; gap:10px; flex:1; padding:10px 16px; border:1px solid transparent; border-radius:6px; font-size:14px; font-weight:600; cursor:pointer; }
.doc-tabs__list button > span { color:var(--vp-c-text-3); }
.doc-tabs__list button[aria-selected=true] { border-color:var(--vp-c-divider); background:var(--vp-c-bg); color:var(--vp-c-brand-1); }
.doc-tabs__panel { padding:20px 24px; font-size:14px; }
.doc-tabs__panel > p:first-child { margin-top:0; }
.doc-tabs__panel > p:last-child { margin-bottom:0; }
</style>
