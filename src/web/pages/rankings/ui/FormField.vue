<script setup lang="ts">
import BaseFormRow from '@/shared/ui/form/BaseFormRow.vue';
/** Compatibility wrapper while ranking fields adopt the shared field contract. */
defineProps<{ label: string; group?: boolean; hint?: string; error?: string }>();
</script>
<template>
  <BaseFormRow
    class="ranking-form-field"
    :label="label"
    :group="group"
    :hint="hint"
    :error="error"
    stacked
  >
    <slot />
  </BaseFormRow>
</template>
<style scoped>
.ranking-form-field { flex: 1; min-width: min(140px, 100%); }
:deep(input), :deep(select) { box-sizing: border-box; width: 100%; min-height: var(--control-height-lg); padding: var(--space-2) var(--space-4); color: var(--color-text); background: var(--color-bg-field); border: var(--border-width) solid var(--color-border); border-radius: var(--radius-sm); font: inherit; }
:deep(input:focus-visible), :deep(select:focus-visible) { outline: 2px solid var(--color-border-focus); outline-offset: 1px; }
@media (any-pointer: coarse) { :deep(input), :deep(select) { min-height: 44px; } }
</style>
