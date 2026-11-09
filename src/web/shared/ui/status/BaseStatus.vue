<script setup lang="ts">
import type { StatusTone } from './status-types.ts';
withDefaults(defineProps<{ label: string; tone?: StatusTone }>(), { tone: 'neutral' });
</script>
<template>
  <span
    class="base-status"
    :class="`base-status--${tone}`"
  >
    <span
      class="base-status__mark"
      aria-hidden="true"
    >●</span>
    <span class="base-status__label">{{ label }}</span>
  </span>
</template>
<style scoped>
.base-status { display: inline-flex; align-items: baseline; gap: var(--space-3); min-width: 0; color: var(--color-text); font: inherit; }
.base-status__mark { flex: none; color: var(--color-text-muted); font-size: var(--font-size-xs); }
.base-status__label { overflow-wrap: anywhere; }
.base-status--info .base-status__mark { color: var(--color-border-focus); }
.base-status--success .base-status__mark { color: var(--color-success); }
.base-status--warning .base-status__mark { color: var(--color-warning); }
.base-status--error .base-status__mark { color: var(--color-danger-hover); }
</style>
