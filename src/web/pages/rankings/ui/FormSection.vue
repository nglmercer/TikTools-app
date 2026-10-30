<script setup lang="ts">
defineProps<{ title: string }>();
</script>

<template>
  <section class="form-section">
    <h3>{{ title }}</h3>
    <slot />
  </section>
</template>

<style scoped>
.form-section { display: grid; gap: var(--space-3); padding-bottom: var(--space-4); border-bottom: 1px solid var(--color-border); }
.form-section h3 { margin: 0; font-size: 14px; }
.form-section :deep(.row) { display: flex; flex-wrap: wrap; align-items: end; gap: var(--space-3); }
</style>
