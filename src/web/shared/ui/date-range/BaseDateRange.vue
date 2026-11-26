<script setup lang="ts">
import type { ControlSize } from '../types.ts';
import BaseDateField from '../date-field/BaseDateField.vue';

/**
 * Paired start/end date fields with the cross-constraints wired in:
 * start never passes end and vice versa. Inline on wide layouts,
 * stacked full-width on narrow ones.
 */
withDefaults(
  defineProps<{
    fromLabel: string;
    toLabel: string;
    size?: ControlSize;
    disabled?: boolean;
  }>(),
  { size: 'md', disabled: false },
);

const start = defineModel<string>('start', { required: true });
const end = defineModel<string>('end', { required: true });
</script>

<template>
  <div class="base-date-range">
    <label class="base-date-range__field">
      <span class="base-date-range__label">{{ fromLabel }}</span>
      <BaseDateField
        v-model="start"
        :max="end"
        :size="size"
        :disabled="disabled"
      />
    </label>
    <label class="base-date-range__field">
      <span class="base-date-range__label">{{ toLabel }}</span>
      <BaseDateField
        v-model="end"
        :min="start"
        :size="size"
        :disabled="disabled"
      />
    </label>
  </div>
</template>

<style scoped>
.base-date-range {
  display: flex;
  align-items: center;
  gap: var(--space-5);
}

.base-date-range__field {
  display: inline-flex;
  align-items: center;
  gap: var(--space-3);
  color: var(--color-text-muted);
}

@media (width < 600px) {
  .base-date-range {
    flex-direction: column;
    align-items: stretch;
    gap: var(--space-2);
    width: 100%;
  }

  .base-date-range__field {
    display: grid;
    grid-template-columns: 4.5rem minmax(0, 1fr);
  }
}
</style>
