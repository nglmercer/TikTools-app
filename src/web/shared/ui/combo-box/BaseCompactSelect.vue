<script setup lang="ts">
import { computed } from 'vue';

const props = defineProps<{
  modelValue: string;
  options: readonly { value: string; label: string; short?: string }[];
  ariaLabel: string;
  title?: string;
  testId?: string;
}>();
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();

const selected = computed(() => props.options.find((option) => option.value === props.modelValue));

function onChange(event: Event): void {
  emit('update:modelValue', (event.target as HTMLSelectElement).value);
}
</script>

<template>
  <label
    class="compact-select"
    :title="title ?? selected?.label"
  >
    <span class="compact-select__value">{{ selected?.short ?? selected?.label ?? modelValue }}</span>
    <svg
      class="compact-select__caret"
      viewBox="0 0 12 12"
      aria-hidden="true"
    ><path
      d="M2.5 4.5 6 8l3.5-3.5"
      fill="none"
      stroke="currentColor"
      stroke-width="1.4"
    /></svg>
    <select
      :data-testid="testId"
      :value="modelValue"
      :aria-label="ariaLabel"
      @change="onChange"
    >
      <option
        v-for="option in options"
        :key="option.value"
        :value="option.value"
      >{{ option.label }}</option>
    </select>
  </label>
</template>

<style scoped>
.compact-select {
  position: relative;
  display: inline-flex;
  align-items: center;
  gap: 4px;
  max-width: 100%;
  padding: 4px 8px;
  border: 1px solid var(--vp-c-divider, var(--color-border, currentColor));
  border-radius: 5px;
  background: var(--vp-c-bg, transparent);
  color: inherit;
  font: 11px/1.4 var(--vp-font-family-base, var(--font-family, inherit));
  white-space: nowrap;
  cursor: pointer;
}

.compact-select__value { font-variant-numeric: tabular-nums; }
.compact-select__caret { width: 10px; height: 10px; opacity: .7; }
.compact-select select {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  opacity: 0;
  cursor: pointer;
}
.compact-select:focus-within { outline: 2px solid var(--vp-c-brand-1, var(--color-border-focus, currentColor)); outline-offset: 2px; }
</style>
