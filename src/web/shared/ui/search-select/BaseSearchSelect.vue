<script setup lang="ts">
import { computed, nextTick, ref, useId, useTemplateRef } from 'vue';
import { useI18n } from 'vue-i18n';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import { stepEnabledIndex, firstEnabledIndex, lastEnabledIndex } from '@/shared/lib/list-navigation.ts';
import { useAnchoredPosition } from '@/shared/lib/use-anchored-position.ts';
import { useOutsidePointer } from '@/shared/lib/use-outside-pointer.ts';
import { useSplitAttrs } from '@/shared/lib/use-split-attrs.ts';
import type { SearchOption } from './search-option.ts';
import { filterSearchOptions } from './search-option.ts';

defineOptions({ inheritAttrs: false });

const props = withDefaults(
  defineProps<{
    options: readonly SearchOption[];
    placeholder?: string;
    allowCustom?: boolean;
    disabled?: boolean;
    invalid?: boolean;
  }>(),
  { placeholder: undefined, allowCustom: false },
);

const model = defineModel<string>({ default: '' });
const { t } = useI18n();
const { rootAttrs, controlAttrs } = useSplitAttrs();
const id = useId();
const root = useTemplateRef('root');
const input = useTemplateRef('input');
const listbox = useTemplateRef('listbox');
const open = ref(false);
const query = ref<string | null>(null);
const activeIndex = ref(-1);

const selected = computed(() => props.options.find((option) => option.value === model.value));
const text = computed(() => query.value ?? selected.value?.label ?? model.value);
const filtered = computed(() => filterSearchOptions(props.options, query.value ?? ''));
const optionId = (index: number): string => `${id}-option-${index}`;

const { style } = useAnchoredPosition(root, listbox, open, { offset: 1, matchAnchorWidth: true });
useOutsidePointer([root, listbox], open, () => close(true));

function show(): void {
  if (props.disabled || open.value) return;
  open.value = true;
  activeIndex.value = Math.max(0, filtered.value.findIndex((option) => option.value === model.value));
}

function close(commit: boolean): void {
  open.value = false;
  if (commit && props.allowCustom && query.value !== null) model.value = query.value.trim();
  query.value = null;
}

function select(index: number): void {
  const option = filtered.value[index];
  if (!option || option.disabled) return;
  model.value = option.value;
  query.value = null;
  open.value = false;
  input.value?.focus();
}

function setActive(index: number): void {
  activeIndex.value = index;
  void nextTick(() => listbox.value?.querySelector(`#${CSS.escape(optionId(index))}`)?.scrollIntoView({ block: 'nearest' }));
}

function onInput(event: Event): void {
  query.value = (event.target as HTMLInputElement).value;
  show();
  activeIndex.value = firstEnabledIndex(filtered.value);
}

function onKeydown(event: KeyboardEvent): void {
  const actions: Record<string, () => void> = {
    ArrowDown: () => (open.value ? setActive(stepEnabledIndex(filtered.value, activeIndex.value, 1)) : show()),
    ArrowUp: () => open.value && setActive(stepEnabledIndex(filtered.value, activeIndex.value, -1)),
    Home: () => open.value && setActive(firstEnabledIndex(filtered.value)),
    End: () => open.value && setActive(lastEnabledIndex(filtered.value)),
    Enter: () => (open.value && activeIndex.value >= 0 ? select(activeIndex.value) : close(true)),
    Escape: () => close(false),
  };
  const action = actions[event.key];
  if (!action) return;
  if (event.key !== 'Home' && event.key !== 'End') event.preventDefault();
  else if (open.value) event.preventDefault();
  action();
}
</script>

<template>
  <span
    ref="root"
    v-bind="rootAttrs"
    class="base-search-select"
    :class="{ 'base-search-select--invalid': invalid, 'base-search-select--disabled': disabled }"
  >
    <span
      v-if="$slots.prefix"
      class="base-search-select__prefix"
    ><slot name="prefix" /></span>
    <img
      v-if="selected?.iconUrl && query === null"
      class="base-search-select__icon"
      :src="selected.iconUrl"
      alt=""
      referrerpolicy="no-referrer"
    >
    <input
      ref="input"
      v-bind="controlAttrs"
      class="base-search-select__input"
      type="text"
      role="combobox"
      autocomplete="off"
      aria-autocomplete="list"
      :aria-expanded="open"
      :aria-controls="`${id}-listbox`"
      :aria-activedescendant="open && activeIndex >= 0 ? optionId(activeIndex) : undefined"
      :value="text"
      :placeholder="placeholder"
      :disabled="disabled"
      @input="onInput"
      @keydown="onKeydown"
      @click="show"
    >
    <button
      class="base-search-select__toggle"
      type="button"
      tabindex="-1"
      :aria-label="t('common.showOptions')"
      :disabled="disabled"
      @click="open ? close(false) : (show(), input?.focus())"
    >
      <IconChevronDown :size="12" />
    </button>
    <Teleport to="body">
      <ul
        v-if="open"
        :id="`${id}-listbox`"
        ref="listbox"
        class="base-search-select__listbox"
        role="listbox"
        :style="style"
      >
        <li
          v-for="(option, index) in filtered"
          :id="optionId(index)"
          :key="option.value"
          class="base-search-select__option"
          :class="{ 'base-search-select__option--active': index === activeIndex }"
          role="option"
          :aria-selected="option.value === model"
          :aria-disabled="option.disabled || undefined"
          @pointerenter="!option.disabled && (activeIndex = index)"
          @pointerdown.prevent
          @click="select(index)"
        >
          <img
            v-if="option.iconUrl"
            class="base-search-select__icon"
            :src="option.iconUrl"
            alt=""
            loading="lazy"
            referrerpolicy="no-referrer"
          >
          <span class="base-search-select__label">{{ option.label }}</span>
          <span
            v-if="option.detail"
            class="base-search-select__detail"
          >{{ option.detail }}</span>
        </li>
        <li
          v-if="!filtered.length"
          class="base-search-select__empty"
          role="presentation"
        >
          {{ allowCustom ? t('common.useCustomValue', { value: query ?? '' }) : t('common.noOptions') }}
        </li>
      </ul>
    </Teleport>
  </span>
</template>

<style scoped>
.base-search-select {
  display: inline-flex;
  align-items: center;
  min-width: 0;
  height: var(--control-height-md);
  padding-left: var(--control-padding-x);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-field);
}

.base-search-select:hover {
  border-color: var(--color-border-strong);
}

.base-search-select:focus-within {
  border-color: var(--color-border-focus);
  box-shadow: var(--focus-ring);
}

.base-search-select--invalid {
  border-color: var(--color-danger);
}

.base-search-select--disabled {
  border-color: var(--color-border-subtle);
}

.base-search-select__input {
  flex: 1;
  min-width: 0;
  height: 100%;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  outline: none;
  box-shadow: none;
}

.base-search-select__toggle {
  display: flex;
  align-items: center;
  justify-content: center;
  width: var(--control-height-md);
  height: 100%;
  padding: 0;
  border: 0;
  border-left: var(--border-width) solid var(--color-border);
  border-radius: 0;
  background: var(--color-bg-control);
  color: var(--color-text);
  cursor: pointer;
  transform: none;
}

.base-search-select__toggle:hover {
  background: var(--color-bg-control-hover);
}

.base-search-select__icon {
  flex: none;
  width: 18px;
  height: 18px;
  margin-right: var(--space-3);
  object-fit: contain;
}

.base-search-select__prefix {
  display: flex;
  flex: none;
  align-items: center;
  margin-right: var(--space-2);
  color: var(--color-text-muted);
}

.base-search-select__listbox {
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
}

.base-search-select__option {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--control-padding-x);
  cursor: pointer;
  user-select: none;
}

.base-search-select__option--active {
  background: var(--color-bg-selected);
  color: var(--color-text-on-accent);
}

.base-search-select__option[aria-disabled='true'] {
  background: transparent;
  color: var(--color-text-disabled);
  cursor: default;
}

.base-search-select__label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.base-search-select__detail {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.base-search-select__option--active .base-search-select__detail {
  color: inherit;
}

.base-search-select__empty {
  padding: var(--space-3) var(--control-padding-x);
  color: var(--color-text-subtle);
}
</style>
