<script setup lang="ts" generic="T extends string | number">
import { computed, toRef } from 'vue';
import { useRovingSelection } from '@/shared/lib/use-roving-selection.ts';
import type { ControlSize, SelectOption } from '../types.ts';

const props = withDefaults(
  defineProps<{
    options: readonly SelectOption<T>[];
    size?: ControlSize;
    disabled?: boolean;
  }>(),
  { size: 'md' },
);

const model = defineModel<T>({ required: true });
const selectedIndex = computed(() => props.options.findIndex((option) => option.value === model.value));
const { focusIndex, select, onKeydown, setElement } = useRovingSelection(toRef(props, 'options'), selectedIndex, (index) => {
  if (!props.disabled) model.value = props.options[index]!.value;
});
</script>

<template>
  <div
    class="base-segmented"
    :class="`base-segmented--${size}`"
    role="radiogroup"
    :aria-disabled="disabled || undefined"
    @keydown="onKeydown"
  >
    <button
      v-for="(option, index) in options"
      :key="String(option.value)"
      :ref="(element) => setElement(index, element)"
      class="base-segmented__option"
      type="button"
      role="radio"
      :aria-checked="index === selectedIndex"
      :tabindex="index === focusIndex ? 0 : -1"
      :disabled="disabled || option.disabled"
      @click="select(index)"
    >
      <component
        :is="option.icon"
        v-if="option.icon"
        :size="14"
        aria-hidden="true"
      />
      {{ option.label }}
    </button>
  </div>
</template>

<style scoped>
.base-segmented {
  display: inline-flex;
  height: var(--control-height-md);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-control-active);
}

.base-segmented--sm {
  height: var(--control-height-sm);
}

.base-segmented__option {
  padding: 0 var(--space-5);
  border: 0;
  border-radius: 0;
  background: transparent;
  color: var(--color-text-muted);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-regular);
  white-space: nowrap;
  cursor: pointer;
  outline: none;
  transform: none;
}

.base-segmented--sm .base-segmented__option {
  padding: 0 var(--space-4);
  font-size: var(--font-size-sm);
}

.base-segmented__option + .base-segmented__option {
  border-left: var(--border-width) solid var(--color-border);
}

.base-segmented__option:hover {
  background: var(--color-bg-hover);
  color: var(--color-text);
}

.base-segmented__option[aria-checked='true'] {
  background: var(--color-bg-selected);
  color: var(--color-text-on-accent);
}

.base-segmented__option:focus-visible {
  box-shadow: inset var(--focus-ring);
}

.base-segmented__option:disabled {
  background: transparent;
  color: var(--color-text-disabled);
  cursor: default;
}
</style>
