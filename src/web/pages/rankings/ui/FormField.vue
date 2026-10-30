<script setup lang="ts">
/** `group` labels a composite control (e.g. a segmented picker) instead of wrapping a single input. */
defineProps<{ label: string; group?: boolean }>();
</script>

<template>
  <component
    :is="group ? 'div' : 'label'"
    class="form-field"
    :role="group ? 'group' : undefined"
    :aria-label="group ? label : undefined"
  >
    <span class="form-field__label">{{ label }}</span>
    <slot />
  </component>
</template>

<style scoped>
.form-field { display: grid; gap: var(--space-1); flex: 1; min-width: 140px; }
.form-field__label { font-size: 12px; color: var(--color-text-muted); }
:slotted(input), :slotted(select) { box-sizing: border-box; width: 100%; min-height: 32px; padding: 4px 8px; color: var(--color-text); background: var(--color-bg-control); border: 1px solid var(--color-border); border-radius: var(--radius-sm); }
</style>
