<script setup lang="ts">
withDefaults(defineProps<{ title: string; tone?: 'tip' | 'info' | 'warning' }>(), { tone: 'info' });
</script>
<template>
  <aside class="doc-callout" :class="`doc-callout--${tone}`" :aria-label="title">
    <strong>{{ title }}</strong><slot />
  </aside>
</template>
<style>
.doc-callout { padding:18px 22px; margin:24px 0; background:var(--vp-c-brand-soft); border:1px solid var(--vp-c-divider); border-left:3px solid var(--vp-c-brand-1); border-radius:0 10px 10px 0; font-size:14px; }
.doc-callout > strong { display:block; color:var(--vp-c-text-1); }
.doc-callout > p { margin:6px 0 0; }
.doc-callout--tip { border-left-color:var(--vp-c-green-1); }
.doc-callout--warning { border-left-color:var(--vp-c-yellow-1); background:var(--vp-c-warning-soft); }
</style>
