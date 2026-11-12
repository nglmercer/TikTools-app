<script setup lang="ts" generic="T extends string | number">
import { computed, nextTick, ref, shallowRef, useId, useTemplateRef } from 'vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import { useAnchoredPosition } from '@/shared/lib/use-anchored-position.ts';
import { useOutsidePointer } from '@/shared/lib/use-outside-pointer.ts';
import { useSplitAttrs } from '@/shared/lib/use-split-attrs.ts';
import type { ControlSize, SelectOption } from '../types.ts';
import { firstEnabledIndex, lastEnabledIndex, stepEnabledIndex, typeaheadIndex } from '@/shared/lib/list-navigation.ts';

defineOptions({ inheritAttrs: false });

const props = withDefaults(
  defineProps<{
    options: readonly SelectOption<T>[];
    placeholder?: string;
    size?: ControlSize;
    disabled?: boolean;
  }>(),
  { placeholder: '', size: 'md' },
);

const model = defineModel<T>();

const { rootAttrs, controlAttrs } = useSplitAttrs();
const id = useId();
const trigger = useTemplateRef('trigger');
const listbox = useTemplateRef('listbox');
const open = ref(false);
const teleportTarget = shallowRef<HTMLElement | string>('body');
const activeIndex = ref(-1);

const { style } = useAnchoredPosition(trigger, listbox, open, { offset: 1, matchAnchorWidth: true });
useOutsidePointer([trigger, listbox], open, () => close());

const selectedIndex = computed(() => props.options.findIndex((option) => option.value === model.value));
const selectedLabel = computed(() => props.options[selectedIndex.value]?.label);
const optionId = (index: number): string => `${id}-option-${index}`;

function scrollActiveIntoView(): void {
  void nextTick(() => {
    listbox.value?.querySelector<HTMLElement>(`#${CSS.escape(optionId(activeIndex.value))}`)?.scrollIntoView({
      block: 'nearest',
    });
  });
}

function setActive(index: number): void {
  activeIndex.value = index;
  scrollActiveIntoView();
}

function show(): void {
  if (props.disabled || open.value) return;
  teleportTarget.value = trigger.value?.closest('dialog[open]') ?? 'body';
  open.value = true;
  setActive(selectedIndex.value >= 0 ? selectedIndex.value : firstEnabledIndex(props.options));
}

function close(): void {
  open.value = false;
}

function select(index: number): void {
  const option = props.options[index];
  if (!option || option.disabled) return;
  model.value = option.value;
  close();
  trigger.value?.focus();
}

let typeahead = '';
let typeaheadTimer: ReturnType<typeof setTimeout> | undefined;

function onTypeahead(key: string): void {
  clearTimeout(typeaheadTimer);
  typeahead += key;
  typeaheadTimer = setTimeout(() => {
    typeahead = '';
  }, 500);
  const from = open.value ? activeIndex.value : selectedIndex.value;
  const index = typeaheadIndex(props.options, typeahead, from);
  if (index < 0) return;
  if (open.value) setActive(index);
  else model.value = props.options[index]!.value;
}

function onKeydown(event: KeyboardEvent): void {
  if (props.disabled) return;
  const { key } = event;
  if (!open.value) {
    if (['ArrowDown', 'ArrowUp', 'Enter', ' '].includes(key)) {
      event.preventDefault();
      show();
    } else if (key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      onTypeahead(key);
    }
    return;
  }
  const actions: Record<string, () => void> = {
    ArrowDown: () => setActive(stepEnabledIndex(props.options, activeIndex.value, 1)),
    ArrowUp: () => setActive(stepEnabledIndex(props.options, activeIndex.value, -1)),
    Home: () => setActive(firstEnabledIndex(props.options)),
    End: () => setActive(lastEnabledIndex(props.options)),
    Enter: () => select(activeIndex.value),
    ' ': () => select(activeIndex.value),
    Escape: close,
  };
  const action = actions[key];
  if (action) {
    event.preventDefault();
    action();
  } else if (key === 'Tab') {
    close();
  } else if (key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
    onTypeahead(key);
  }
}

function toggle(): void {
  if (open.value) close();
  else show();
}

defineExpose({ focus: () => trigger.value?.focus() });
</script>

<template>
  <span
    v-bind="rootAttrs"
    class="base-combo-box"
  >
    <button
      ref="trigger"
      v-bind="controlAttrs"
      class="base-combo-box__trigger"
      :class="[`base-combo-box__trigger--${size}`, { 'base-combo-box__trigger--open': open }]"
      type="button"
      role="combobox"
      aria-haspopup="listbox"
      :aria-expanded="open"
      :aria-controls="`${id}-listbox`"
      :aria-activedescendant="open && activeIndex >= 0 ? optionId(activeIndex) : undefined"
      :disabled="disabled"
      @click="toggle"
      @keydown="onKeydown"
    >
      <span
        v-if="$slots.prefix"
        class="base-combo-box__prefix"
      ><slot name="prefix" /></span>
      <span
        class="base-combo-box__value"
        :class="{ 'base-combo-box__value--placeholder': !selectedLabel }"
      >{{ selectedLabel ?? placeholder }}</span>
      <span class="base-combo-box__arrow">
        <IconChevronDown :size="12" />
      </span>
    </button>
    <Teleport :to="teleportTarget">
      <ul
        v-if="open"
        :id="`${id}-listbox`"
        ref="listbox"
        class="base-combo-box__listbox"
        role="listbox"
        tabindex="-1"
        :style="style"
      >
        <li
          v-for="(option, index) in options"
          :id="optionId(index)"
          :key="String(option.value)"
          class="base-combo-box__option"
          :class="{ 'base-combo-box__option--active': index === activeIndex }"
          role="option"
          :aria-selected="index === selectedIndex"
          :aria-disabled="option.disabled || undefined"
          @pointerenter="!option.disabled && (activeIndex = index)"
          @pointerdown.prevent
          @click="select(index)"
        >
          {{ option.label }}
        </li>
      </ul>
    </Teleport>
  </span>
</template>

<style scoped>
.base-combo-box {
  display: inline-flex;
  min-width: 0;
}

.base-combo-box__trigger {
  display: inline-flex;
  flex: 1;
  align-items: stretch;
  min-width: 0;
  height: var(--control-height-md);
  padding: 0;
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-control);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-regular);
  text-align: left;
  cursor: pointer;
  outline: none;
  transform: none;
  transition:
    background-color var(--duration-fast),
    border-color var(--duration-fast);
}

.base-combo-box__trigger:hover {
  border-color: var(--color-border-strong);
  background: var(--color-bg-control-hover);
}

.base-combo-box__trigger:focus-visible,
.base-combo-box__trigger--open {
  border-color: var(--color-border-focus);
  box-shadow: var(--focus-ring);
}

.base-combo-box__trigger--sm {
  height: var(--control-height-sm);
  font-size: var(--font-size-sm);
}

.base-combo-box__trigger:disabled {
  border-color: var(--color-border-subtle);
  background: var(--color-bg-control-active);
  color: var(--color-text-disabled);
  cursor: default;
}

.base-combo-box__value {
  display: flex;
  flex: 1;
  align-items: center;
  min-width: 0;
  padding: 0 var(--control-padding-x);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.base-combo-box__prefix {
  display: inline-flex;
  flex: none;
  align-items: center;
  padding-left: var(--control-padding-x);
  color: var(--color-text-muted);
}

.base-combo-box__prefix + .base-combo-box__value {
  padding-left: var(--space-2);
}

.base-combo-box__value--placeholder {
  color: var(--color-text-subtle);
}

.base-combo-box__arrow {
  display: flex;
  flex: none;
  align-items: center;
  justify-content: center;
  width: var(--control-height-md);
  border-left: var(--border-width) solid var(--color-border);
}

.base-combo-box__trigger--sm .base-combo-box__arrow {
  width: var(--control-height-sm);
}

.base-combo-box__listbox {
  z-index: var(--z-dropdown);
  max-height: 320px;
  margin: 0;
  padding: var(--space-1) 0;
  overflow-y: auto;
  border: var(--border-width) solid var(--color-border-strong);
  border-radius: var(--radius-sm);
  background: var(--color-bg-overlay);
  box-shadow: var(--shadow-popup);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  list-style: none;
  outline: none;
}

.base-combo-box__option {
  padding: var(--space-2) var(--control-padding-x);
  white-space: nowrap;
  cursor: pointer;
  user-select: none;
}

.base-combo-box__option[aria-selected='true'] {
  font-weight: var(--font-weight-semibold);
}

.base-combo-box__option--active {
  background: var(--color-bg-selected);
  color: var(--color-text-on-accent);
}

.base-combo-box__option[aria-disabled='true'] {
  background: transparent;
  color: var(--color-text-disabled);
  cursor: default;
}
</style>
