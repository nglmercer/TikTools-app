<script setup lang="ts">
defineProps<{ title: string }>();
</script>
<template>
  <aside class="doc-requirements" :aria-label="title"><strong>{{ title }}</strong><slot /></aside>
</template>
<style>
.doc-requirements { margin:24px 0; padding:20px 22px; border:1px solid var(--vp-c-divider); border-radius:10px; background:var(--vp-c-bg-soft); font-size:14px; }
.doc-requirements > strong { display:block; margin-bottom:12px; }
.doc-requirements ul { display:grid; gap:8px; list-style:none; padding:0; margin:0; }
.doc-requirements li { position:relative; padding-left:24px; margin:0; color:var(--vp-c-text-2); }
.doc-requirements li::before { content:''; width:9px; height:9px; position:absolute; left:1px; top:.6em; border:1.5px solid var(--vp-c-brand-1); border-radius:3px; }
</style>
