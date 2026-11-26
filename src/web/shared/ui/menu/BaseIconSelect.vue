<script setup lang="ts">
import { computed, ref, type Component } from 'vue';
import type { Placement } from '@/shared/lib/anchored-position.ts';
import BaseMenu from './BaseMenu.vue';
import type { MenuAction } from './menu-types.ts';

/** A label-like picker built on BaseMenu: no native select, icons and checks included. */
defineOptions({ inheritAttrs: false });
const props = defineProps<{
  modelValue: string;
  options: readonly { value: string; label: string; short?: string; icon?: Component; color?: string }[];
  label: string;
  placement?: Placement;
  testId?: string;
}>();
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();

const open = ref(false);
const trigger = ref<HTMLElement | null>(null);
const selected = computed(() => props.options.find((option) => option.value === props.modelValue));
const items = computed<MenuAction[]>(() => props.options.map((option) => ({
  id: option.value,
  label: option.label,
  icon: option.icon,
  color: option.color,
  checked: option.value === props.modelValue,
})));

function select(action: MenuAction): void {
  emit('update:modelValue', action.id);
}
</script>

<template>
  <button
    ref="trigger"
    v-bind="$attrs"
    type="button"
    class="icon-select"
    :data-testid="testId"
    :aria-label="label"
    :aria-expanded="open"
    aria-haspopup="menu"
    @click="open = !open"
  >
    <component
      :is="selected?.icon"
      v-if="selected?.icon"
      class="icon-select__icon"
      :style="selected?.color ? { color: selected.color } : undefined"
      aria-hidden="true"
    />
    <span class="icon-select__value">{{ selected?.short ?? selected?.label ?? modelValue }}</span>
    <svg
      class="icon-select__caret"
      viewBox="0 0 12 12"
      aria-hidden="true"
    ><path
      d="M2.5 4.5 6 8l3.5-3.5"
      fill="none"
      stroke="currentColor"
      stroke-width="1.4"
    /></svg>
  </button>
  <BaseMenu
    v-model:open="open"
    :items="items"
    :anchor="trigger"
    :placement="placement ?? 'bottom-end'"
    @select="select"
  />
</template>

<style scoped>
.icon-select {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: 100%;
  padding: 5px 9px;
  border: 1px solid var(--vp-c-divider, var(--color-border, currentColor));
  border-radius: 5px;
  background: var(--vp-c-bg, transparent);
  color: inherit;
  font: 11px/1.4 var(--vp-font-family-base, var(--font-family, inherit));
  white-space: nowrap;
  cursor: pointer;
}

.icon-select:hover,
.icon-select[aria-expanded='true'] { border-color: var(--vp-c-brand-1, var(--color-border-focus, currentColor)); }
.icon-select__value { min-width: 0; overflow: hidden; text-overflow: ellipsis; font-variant-numeric: tabular-nums; }
.icon-select__icon,
.icon-select__caret { width: 12px; height: 12px; flex: none; }
.icon-select__caret { opacity: .7; }
.icon-select:focus-visible { outline: 2px solid var(--vp-c-brand-1, var(--color-border-focus, currentColor)); outline-offset: 2px; }
</style>
