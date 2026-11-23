<script setup lang="ts">
import BaseStatus from './BaseStatus.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import { dismissSpotlight, vSpotlight } from '@/shared/ui/spotlight/v-spotlight.ts';
import type { StatusTone } from './status-types.ts';
withDefaults(defineProps<{ label: string; actionLabel: string; tone?: StatusTone; disabled?: boolean; detail?: string }>(), { tone: 'neutral', detail: '' });
</script>
<template>
  <button
    v-tooltip="detail"
    v-spotlight="Boolean(detail)"
    type="button"
    class="base-status-action"
    :disabled="disabled"
    :aria-label="`${label}. ${actionLabel}`"
    @click="dismissSpotlight()"
  >
    <BaseStatus
      :label="label"
      :tone="tone"
    />
    <span class="base-status-action__hint">{{ actionLabel }} ↗</span>
  </button>
</template>
<style scoped>
.base-status-action { display: inline-flex; align-items: center; flex-wrap: wrap; gap: var(--space-3); max-width: 100%; padding: var(--space-2); border: var(--border-width) solid transparent; border-radius: var(--radius-sm); background: transparent; color: var(--color-text); font: inherit; text-align: left; cursor: pointer; }
.base-status-action__hint { color: var(--color-text-muted); text-decoration: underline; text-underline-offset: 3px; white-space: nowrap; }
.base-status-action:hover { border-color: var(--color-border-selected); background: var(--color-bg-selected); }
.base-status-action:hover .base-status-action__hint,.base-status-action:focus-visible .base-status-action__hint { color: var(--color-accent-hover); }
.base-status-action:active { background: var(--color-accent); }
.base-status-action:active .base-status-action__hint { color: var(--color-text-on-accent); }
.base-status-action:focus-visible { outline: var(--border-width) solid var(--color-border-focus); box-shadow: var(--focus-ring); }
.base-status-action:disabled :deep(.base-status), .base-status-action:disabled .base-status-action__hint { color: var(--color-text-disabled); }
.base-status-action:disabled { color: var(--color-text-disabled); cursor: default; }
@media (any-pointer: coarse) { .base-status-action { min-height: var(--control-height-touch); } }
</style>
