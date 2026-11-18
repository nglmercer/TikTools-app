<script setup lang="ts">
import { computed, ref, useId } from 'vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';

/**
 * The single disclosure primitive used in this app. Raw <details>/<summary>
 * are forbidden (see eslint.config.js): native summaries cannot be styled or
 * placed inline consistently, and this component keeps one accessible
 * button/region pair everywhere.
 *
 * `open` stays compatible with `v-model:open`; when it is omitted the
 * component owns its state and `initiallyOpen` seeds it (like a native
 * `details open` attribute). `inline` renders inside a sentence / <li>.
 */
const props = withDefaults(defineProps<{
  title?: string;
  count?: number;
  initiallyOpen?: boolean;
  open?: boolean;
  inline?: boolean;
  dense?: boolean;
  toggleClass?: string;
}>(), { title: '', count: undefined, open: undefined, initiallyOpen: false, inline: false, dense: false, toggleClass: '' });

const emit = defineEmits<{ 'update:open': [value: boolean] }>();
const localOpen = ref(props.initiallyOpen);
const expanded = computed({
  get: () => props.open ?? localOpen.value,
  set: (value: boolean) => {
    localOpen.value = value;
    emit('update:open', value);
  },
});
const id = useId();
const toggleId = `${id}-toggle`;
</script>

<template>
  <component
    :is="inline ? 'span' : 'section'"
    class="base-disclosure"
    :class="{ 'base-disclosure--inline': inline, 'base-disclosure--dense': dense }"
  >
    <button
      :id="toggleId"
      class="base-disclosure__toggle"
      :class="toggleClass"
      type="button"
      :aria-expanded="expanded"
      :aria-controls="id"
      @click="expanded = !expanded"
    >
      <IconChevronDown
        v-if="expanded"
        aria-hidden="true"
      />
      <IconChevronRight
        v-else
        aria-hidden="true"
      />
      <span class="base-disclosure__label"><slot name="summary">{{ title }}</slot></span>
      <span
        v-if="count !== undefined"
        class="base-disclosure__count"
      >{{ count }}</span>
    </button>
    <div
      v-show="expanded"
      :id="id"
      class="base-disclosure__body"
      role="region"
      :aria-labelledby="toggleId"
    >
      <slot />
    </div>
  </component>
</template>

<style scoped>
.base-disclosure {
  display: block;
  min-width: 0;
  border-top: var(--border-width) solid var(--color-border-subtle);
}

.base-disclosure__toggle {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: var(--space-3);
  width: 100%;
  height: var(--control-height-md);
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--color-text-muted);
  font-family: var(--font-family);
  font-size: var(--font-size-md);
  font-weight: var(--font-weight-semibold);
  text-align: left;
  cursor: pointer;
  outline: none;
  transform: none;
}

.base-disclosure__toggle:hover {
  color: var(--color-text);
}

.base-disclosure__toggle:focus-visible {
  box-shadow: var(--focus-ring);
}

.base-disclosure__label {
  min-width: 0;
}

.base-disclosure__count {
  padding: 0 var(--space-2);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
  font-size: var(--font-size-xs);
}

.base-disclosure__body {
  padding: var(--space-3) 0 var(--space-2);
}

.base-disclosure--dense {
  border-top: 0;
}

.base-disclosure--dense .base-disclosure__toggle {
  height: auto;
  min-height: var(--control-height-sm);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-regular);
}

.base-disclosure--dense .base-disclosure__body {
  padding: var(--space-2) 0;
}

/* Inline inside a sentence or <li>: no block border, auto-sized toggle. */
.base-disclosure--inline {
  display: inline;
  border-top: 0;
}

.base-disclosure--inline .base-disclosure__toggle {
  display: inline-flex;
  width: auto;
  height: auto;
  min-height: 0;
  gap: var(--space-1);
  padding: 0;
  font: inherit;
  font-weight: var(--font-weight-semibold);
  color: inherit;
  vertical-align: baseline;
}

.base-disclosure--inline .base-disclosure__body {
  display: block;
  padding: var(--space-2) 0;
}

.base-disclosure--inline .base-disclosure__toggle svg {
  width: 1em;
  height: 1em;
  flex: none;
}

@media (any-pointer: coarse), (width < 700px) {
  .base-disclosure:not(.base-disclosure--inline) .base-disclosure__toggle {
    min-height: var(--control-height-touch, var(--control-height-md));
  }
}
</style>
