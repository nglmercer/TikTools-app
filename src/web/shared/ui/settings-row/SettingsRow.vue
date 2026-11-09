<script setup lang="ts">
import { useId } from 'vue';
defineProps<{ label: string; description?: string; for?: string }>();
const id = useId();
</script>
<template>
  <div class="settings-row">
    <div class="settings-row__text">
      <component
        :is="$props.for ? 'label' : 'div'"
        :id="`setting-${id}`"
        :for="$props.for"
        class="settings-row__label"
      >
        {{ label }}
      </component>
      <p
        v-if="description"
        :id="`setting-description-${id}`"
        class="settings-row__description"
      >
        {{ description }}
      </p>
      <slot name="status" />
    </div>
    <div
      class="settings-row__control"
      role="group"
      :aria-labelledby="`setting-${id}`"
      :aria-describedby="description ? `setting-description-${id}` : undefined"
    >
      <slot />
    </div>
    <div
      v-if="$slots.details"
      class="settings-row__details"
    >
      <slot name="details" />
    </div>
  </div>
</template>
<style scoped>
.settings-row { display: grid; grid-template-columns: minmax(0, 1fr) minmax(120px, auto); align-items: center; gap: var(--space-4) var(--space-6); padding: var(--space-5) 0; border-bottom: var(--border-width) solid var(--color-border-subtle); }
.settings-row:last-child { border-bottom: 0; }
.settings-row__text,.settings-row__control { min-width: 0; }
.settings-row__label { font-weight: var(--font-weight-semibold); }
.settings-row__description { margin: var(--space-2) 0 0; color: var(--color-text-muted); font-size: var(--font-size-sm); line-height: var(--line-height-normal); }
.settings-row__control { display: flex; flex-wrap: wrap; align-items: center; gap: var(--space-3); }
.settings-row__details { grid-column: 1 / -1; }
@media (width < 600px) { .settings-row { grid-template-columns: minmax(0, 1fr); } }
</style>
