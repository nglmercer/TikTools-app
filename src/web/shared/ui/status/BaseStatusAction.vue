<script setup lang="ts">
import BaseStatus from './BaseStatus.vue';
import type { StatusTone } from './status-types.ts';
withDefaults(defineProps<{ label: string; actionLabel: string; tone?: StatusTone; disabled?: boolean }>(), { tone: 'neutral' });
</script>
<template>
  <button
    type="button"
    class="base-status-action"
    :disabled="disabled"
    :aria-label="`${label}. ${actionLabel}`"
  >
    <BaseStatus
      :label="label"
      :tone="tone"
    />
    <span
      class="base-status-action__hint"
      aria-hidden="true"
    >{{ actionLabel }} ↗</span>
  </button>
</template>
<style scoped>
.base-status-action { display: inline-flex; align-items: baseline; flex-wrap: wrap; gap: var(--space-3); max-width: 100%; padding: var(--space-2); border: var(--border-width) solid transparent; border-radius: var(--radius-sm); background: transparent; color: var(--color-text); font: inherit; text-align: left; cursor: pointer; }
.base-status-action__hint { color: var(--color-text-muted); text-decoration: underline; white-space: nowrap; }
.base-status-action:hover { background: var(--color-bg-hover); }
.base-status-action:focus-visible { outline: var(--border-width) solid var(--color-border-focus); box-shadow: var(--focus-ring); }
.base-status-action:disabled :deep(.base-status), .base-status-action:disabled .base-status-action__hint { color: var(--color-text-disabled); }
.base-status-action:disabled { color: var(--color-text-disabled); cursor: default; }
@media (hover: hover) and (pointer: fine) and (min-width: 768px) {
  .base-status-action__hint { opacity: 0; }
  .base-status-action:hover .base-status-action__hint, .base-status-action:focus-visible .base-status-action__hint { opacity: 1; }
}
@media (any-pointer: coarse) { .base-status-action { min-height: var(--control-height-touch); } .base-status-action__hint { opacity: 1; } }
</style>
