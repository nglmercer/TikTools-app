<script setup lang="ts">
import { computed, onBeforeUnmount, ref, useTemplateRef } from 'vue';
import { useI18n } from 'vue-i18n';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronUp from '@/shared/ui/icon/icons/IconChevronUp.vue';
import IconMinus from '@/shared/ui/icon/icons/IconMinus.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import { useSplitAttrs } from '@/shared/lib/use-split-attrs.ts';
import type { ControlSize } from '../types.ts';
import {
  formatSpinValue,
  normalizeSpinValue,
  parseSpinValue,
  precisionOf,
  stepSpinValue,
  type SpinRange,
} from './spin-value.ts';

defineOptions({ inheritAttrs: false });

const props = withDefaults(
  defineProps<{
    min?: number;
    max?: number;
    step?: number;
    precision?: number;
    suffix?: string;
    size?: ControlSize;
    /** Vertical arrows inside the field (default) or flat −/+ buttons at the trailing edge. */
    orientation?: 'vertical' | 'horizontal';
    disabled?: boolean;
    readonly?: boolean;
  }>(),
  { min: undefined, max: undefined, step: 1, precision: undefined, suffix: undefined, size: 'md', orientation: 'vertical' },
);

const model = defineModel<number>({ default: 0 });
const { t } = useI18n();
const { rootAttrs, controlAttrs } = useSplitAttrs();
const input = useTemplateRef('input');
const draft = ref<string | null>(null);

const range = computed<SpinRange>(() => ({
  min: props.min,
  max: props.max,
  step: props.step,
  precision: props.precision ?? precisionOf(props.step),
}));

const text = computed(() => draft.value ?? formatSpinValue(model.value, range.value.precision));
const editable = computed(() => !props.disabled && !props.readonly);
const atMin = computed(() => props.min !== undefined && model.value <= props.min);
const atMax = computed(() => props.max !== undefined && model.value >= props.max);

function currentValue(): number {
  return (draft.value === null ? null : parseSpinValue(draft.value)) ?? model.value;
}

function commit(): void {
  const parsed = draft.value === null ? null : parseSpinValue(draft.value);
  if (parsed !== null) model.value = normalizeSpinValue(parsed, range.value);
  draft.value = null;
}

function stepBy(direction: number): boolean {
  if (!editable.value) return false;
  const previous = model.value;
  model.value = stepSpinValue(currentValue(), direction, range.value);
  draft.value = null;
  return model.value !== previous;
}

function setTo(value: number | undefined): void {
  if (value === undefined || !editable.value) return;
  model.value = normalizeSpinValue(value, range.value);
  draft.value = null;
}

const keyActions: Record<string, () => void> = {
  ArrowUp: () => void stepBy(1),
  ArrowDown: () => void stepBy(-1),
  PageUp: () => void stepBy(10),
  PageDown: () => void stepBy(-10),
  Home: () => setTo(props.min),
  End: () => setTo(props.max),
  Enter: commit,
  Escape: () => {
    draft.value = null;
  },
};

function onKeydown(event: KeyboardEvent): void {
  const action = keyActions[event.key];
  if (!action) return;
  event.preventDefault();
  action();
}

function onWheel(event: WheelEvent): void {
  if (document.activeElement !== input.value || event.deltaY === 0) return;
  event.preventDefault();
  stepBy(event.deltaY < 0 ? 1 : -1);
}

let repeatTimer: ReturnType<typeof setTimeout> | undefined;

function stopRepeat(): void {
  clearTimeout(repeatTimer);
  repeatTimer = undefined;
}

function startRepeat(direction: number): void {
  stopRepeat();
  input.value?.focus();
  if (!stepBy(direction)) return;
  const repeat = (delay: number): void => {
    repeatTimer = setTimeout(() => {
      if (stepBy(direction)) repeat(50);
    }, delay);
  };
  repeat(400);
}

onBeforeUnmount(stopRepeat);

defineExpose({ focus: () => input.value?.focus() });
</script>

<template>
  <span
    v-bind="rootAttrs"
    class="base-spin-box"
    :class="[`base-spin-box--${size}`, { 'base-spin-box--disabled': disabled, 'base-spin-box--horizontal': orientation === 'horizontal' }]"
  >
    <span
      v-if="$slots.prefix"
      class="base-spin-box__prefix"
    ><slot name="prefix" /></span>
    <input
      ref="input"
      v-bind="controlAttrs"
      class="base-spin-box__input"
      type="text"
      inputmode="decimal"
      role="spinbutton"
      autocomplete="off"
      :value="text"
      :disabled="disabled"
      :readonly="readonly"
      :aria-valuenow="model"
      :aria-valuemin="min"
      :aria-valuemax="max"
      @input="draft = ($event.target as HTMLInputElement).value"
      @blur="commit"
      @keydown="onKeydown"
      @wheel="onWheel"
    >
    <span
      v-if="suffix"
      class="base-spin-box__suffix"
    >{{ suffix }}</span>
    <span
      v-if="orientation === 'horizontal'"
      class="base-spin-box__steppers"
    >
      <button
        class="base-spin-box__step"
        type="button"
        tabindex="-1"
        :aria-label="t('common.decrease')"
        :disabled="!editable || atMin"
        @pointerdown.prevent="startRepeat(-1)"
        @pointerup="stopRepeat"
        @pointerleave="stopRepeat"
        @pointercancel="stopRepeat"
      >
        <IconMinus :size="12" />
      </button>
      <button
        class="base-spin-box__step"
        type="button"
        tabindex="-1"
        :aria-label="t('common.increase')"
        :disabled="!editable || atMax"
        @pointerdown.prevent="startRepeat(1)"
        @pointerup="stopRepeat"
        @pointerleave="stopRepeat"
        @pointercancel="stopRepeat"
      >
        <IconPlus :size="12" />
      </button>
    </span>
    <span
      v-else
      class="base-spin-box__arrows"
    >
      <button
        class="base-spin-box__arrow"
        type="button"
        tabindex="-1"
        :aria-label="t('common.increase')"
        :disabled="!editable || atMax"
        @pointerdown.prevent="startRepeat(1)"
        @pointerup="stopRepeat"
        @pointerleave="stopRepeat"
        @pointercancel="stopRepeat"
      >
        <IconChevronUp :size="10" />
      </button>
      <button
        class="base-spin-box__arrow"
        type="button"
        tabindex="-1"
        :aria-label="t('common.decrease')"
        :disabled="!editable || atMin"
        @pointerdown.prevent="startRepeat(-1)"
        @pointerup="stopRepeat"
        @pointerleave="stopRepeat"
        @pointercancel="stopRepeat"
      >
        <IconChevronDown :size="10" />
      </button>
    </span>
  </span>
</template>

<style scoped>
.base-spin-box {
  display: inline-flex;
  align-items: stretch;
  min-width: 0;
  height: var(--control-height-md);
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-bg-field);
  color: var(--color-text);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  transition: border-color var(--duration-fast);
}

.base-spin-box:hover {
  border-color: var(--color-border-strong);
}

.base-spin-box:focus-within {
  border-color: var(--color-border-focus);
  box-shadow: var(--focus-ring);
}

.base-spin-box--sm {
  height: var(--control-height-sm);
  font-size: var(--font-size-sm);
}

.base-spin-box--disabled,
.base-spin-box--disabled:hover {
  border-color: var(--color-border-subtle);
  color: var(--color-text-disabled);
}

.base-spin-box__input {
  flex: 1;
  width: 0;
  min-width: 3ch;
  height: 100%;
  padding: 0 var(--space-2) 0 var(--control-padding-x);
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  font-variant-numeric: tabular-nums;
  caret-color: var(--color-text);
  outline: none;
  box-shadow: none;
}

.base-spin-box__prefix {
  display: inline-flex;
  flex: none;
  align-items: center;
  padding-left: var(--control-padding-x);
  color: var(--color-text-muted);
}

.base-spin-box__prefix + .base-spin-box__input {
  padding-left: var(--space-2);
}

.base-spin-box__input:disabled {
  color: inherit;
  cursor: default;
}

.base-spin-box__suffix {
  display: inline-flex;
  align-items: center;
  padding-right: var(--space-3);
  color: var(--color-text-muted);
}

.base-spin-box__arrows {
  display: flex;
  flex: none;
  flex-direction: column;
  width: 18px;
  border-left: var(--border-width) solid var(--color-border);
}

.base-spin-box--horizontal .base-spin-box__input {
  text-align: left;
}

.base-spin-box__steppers {
  display: flex;
  flex: none;
  align-self: stretch;
  margin-left: auto;
}

.base-spin-box__step {
  display: flex;
  align-items: center;
  justify-content: center;
  width: calc(var(--control-height-md) - var(--border-width) * 2);
  padding: 0;
  border: 0;
  border-left: var(--border-width) solid var(--color-border);
  border-radius: 0;
  background: transparent;
  color: var(--color-text-muted);
  cursor: pointer;
  transform: none;
}

.base-spin-box--sm .base-spin-box__step {
  width: calc(var(--control-height-sm) - var(--border-width) * 2);
}

.base-spin-box__step:hover {
  background: var(--color-bg-hover);
  color: var(--color-text);
}

.base-spin-box__step:active {
  background: var(--color-bg-control-active);
}

.base-spin-box__step:disabled {
  background: transparent;
  color: var(--color-text-disabled);
  cursor: default;
}

.base-spin-box__arrow {
  display: flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  min-height: 0;
  padding: 0;
  border: 0;
  border-radius: 0;
  background: var(--color-bg-control);
  color: var(--color-text);
  cursor: pointer;
  transform: none;
}

.base-spin-box__arrow + .base-spin-box__arrow {
  border-top: var(--border-width) solid var(--color-border);
}

.base-spin-box__arrow:hover {
  background: var(--color-bg-control-hover);
}

.base-spin-box__arrow:active {
  background: var(--color-bg-control-active);
}

.base-spin-box__arrow:disabled {
  background: var(--color-bg-control-active);
  color: var(--color-text-disabled);
  cursor: default;
}
</style>
