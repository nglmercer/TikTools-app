<script setup lang="ts">
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import type { ControlSize } from '../types.ts';

withDefaults(
  defineProps<{
    label: string;
    /** Longer explanation shown as the tooltip (and described to assistive tech) instead of the label. */
    hint?: string;
    variant?: 'default' | 'ghost' | 'primary' | 'danger';
    size?: ControlSize;
    disabled?: boolean;
    pressed?: boolean;
  }>(),
  { variant: 'ghost', size: 'md', hint: undefined, pressed: undefined },
);
</script>

<template>
  <button
    v-tooltip="hint ?? label"
    class="base-icon-button"
    :class="[`base-icon-button--${variant}`, `base-icon-button--${size}`]"
    type="button"
    :aria-label="label"
    :aria-description="hint"
    :disabled="disabled"
    :aria-pressed="pressed"
  >
    <slot />
  </button>
</template>

<style scoped>
.base-icon-button {
  display: inline-flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: var(--control-height-md);
  height: var(--control-height-md);
  padding: 0;
  border: var(--border-width) solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--color-text);
  cursor: pointer;
  outline: none;
  transform: none;
  transition: background-color var(--duration-fast);
}

.base-icon-button:hover {
  background: var(--color-bg-hover);
}

.base-icon-button:active {
  background: var(--color-bg-control-active);
}

.base-icon-button:focus-visible {
  border-color: var(--color-border-focus);
  box-shadow: var(--focus-ring);
}

.base-icon-button[aria-pressed='true'] {
  border-color: var(--color-border-selected);
  background: var(--color-bg-selected);
}

.base-icon-button--default {
  border-color: var(--color-border);
  background: var(--color-bg-control);
}

.base-icon-button--default:hover {
  border-color: var(--color-border-strong);
  background: var(--color-bg-control-hover);
}

.base-icon-button--primary {
  border-color: var(--color-border-selected);
  background: var(--color-accent);
  color: var(--color-text-on-accent);
}

.base-icon-button--primary:hover {
  border-color: var(--color-border-focus);
  background: var(--color-accent-hover);
}

.base-icon-button--danger {
  color: var(--color-danger-hover);
}

.base-icon-button--sm {
  width: var(--control-height-sm);
  height: var(--control-height-sm);
}

.base-icon-button:disabled {
  background: transparent;
  color: var(--color-text-disabled);
  cursor: default;
}

.base-icon-button--primary:disabled {
  border-color: var(--color-border-subtle);
  background: var(--color-bg-control-active);
}
</style>
