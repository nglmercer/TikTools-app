<script setup lang="ts">
import { useSplitAttrs } from '@/shared/lib/use-split-attrs.ts';

defineOptions({ inheritAttrs: false });
defineProps<{ disabled?: boolean }>();
const model = defineModel<boolean>({ default: false });
const { rootAttrs, controlAttrs } = useSplitAttrs();
</script>

<template>
  <label
    v-bind="rootAttrs"
    class="base-switch"
    :class="{ 'base-switch--disabled': disabled }"
  >
    <input
      v-bind="controlAttrs"
      v-model="model"
      class="base-switch__input"
      type="checkbox"
      role="switch"
      :disabled="disabled"
    >
    <span
      class="base-switch__track"
      aria-hidden="true"
    ><span class="base-switch__thumb" /></span>
    <span v-if="$slots.default"><slot /></span>
  </label>
</template>

<style scoped>
.base-switch { display: inline-flex; align-items: center; gap: var(--space-3); min-height: var(--control-height-sm); cursor: pointer; color: var(--color-text); }
.base-switch__input { position: absolute; width: 1px; height: 1px; opacity: 0; }
.base-switch__track { display: inline-flex; align-items: center; flex: none; width: 32px; height: 18px; padding: 2px; border: var(--border-width) solid var(--color-border-strong); border-radius: 999px; background: var(--color-bg-field); }
.base-switch__thumb { width: 12px; height: 12px; border-radius: 50%; background: var(--color-text-muted); transition: transform var(--duration-fast); }
.base-switch__input:checked + .base-switch__track { border-color: var(--color-border-selected); background: var(--color-accent); }
.base-switch__input:checked + .base-switch__track .base-switch__thumb { transform: translateX(14px); background: var(--color-text-on-accent); }
.base-switch__input:focus-visible + .base-switch__track { outline: 2px solid var(--color-border-focus); outline-offset: 2px; }
.base-switch--disabled { color: var(--color-text-disabled); cursor: default; opacity: .65; }
@media (prefers-reduced-motion: reduce) { .base-switch__thumb { transition: none; } }
</style>
