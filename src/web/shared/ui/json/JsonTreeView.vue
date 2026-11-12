<script setup lang="ts">
import { useI18n } from 'vue-i18n';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconChevronRight from '@/shared/ui/icon/icons/IconChevronRight.vue';
import type { TreeRow } from './json-tree.ts';

defineProps<{ rows: readonly TreeRow[]; wrap: boolean }>();
const emit = defineEmits<{ toggle: [id: string] }>();
const { t } = useI18n();
</script>

<template>
  <div
    class="base-json__tree"
    :class="{ 'base-json__tree--wrap': wrap }"
  >
    <div
      v-for="row in rows"
      :key="row.id"
      class="base-json__row"
      :style="{ '--json-depth': row.depth }"
    >
      <p
        v-if="row.kind === 'more'"
        class="base-json__more"
      >
        {{ t('jsonView.moreItems', { count: row.count }) }}
      </p>
      <button
        v-else-if="row.expandable"
        class="base-json__toggle"
        type="button"
        :aria-expanded="row.open"
        @click="emit('toggle', row.id)"
      >
        <IconChevronDown
          v-if="row.open"
          class="base-json__chevron"
        />
        <IconChevronRight
          v-else
          class="base-json__chevron"
        />
        <span
          v-if="row.label !== null"
          :class="row.labelKind === 'key' ? 'json-token--key' : 'base-json__index'"
        >{{ row.label }}</span>
        <span
          v-if="row.label !== null"
          class="json-token--punct"
        >: </span>
        <span
          v-for="(token, at) in row.tokens"
          :key="at"
          :class="`json-token--${token.kind}`"
        >{{ token.text }}</span>
      </button>
      <span
        v-else
        class="base-json__leaf"
      >
        <span
          v-if="row.label !== null"
          :class="row.labelKind === 'key' ? 'json-token--key' : 'base-json__index'"
        >{{ row.label }}</span>
        <span
          v-if="row.label !== null"
          class="json-token--punct"
        >: </span>
        <span
          v-for="(token, at) in row.tokens"
          :key="at"
          :class="`json-token--${token.kind}`"
        >{{ token.text }}</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.base-json__tree {
  padding: var(--space-2) var(--space-3);
  color: var(--color-text);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  line-height: 1.5;
  white-space: pre;
}

.base-json__tree--wrap {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.base-json__row {
  padding-inline-start: calc(var(--json-depth, 0) * var(--space-6));
}

.base-json__toggle {
  display: inline-flex;
  align-items: baseline;
  max-width: 100%;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  white-space: inherit;
  overflow-wrap: inherit;
  cursor: pointer;
  outline: none;
  transform: none;
}

.base-json__toggle:hover {
  background: var(--color-bg-hover);
}

.base-json__toggle:focus-visible {
  box-shadow: var(--focus-ring);
}

.base-json__chevron {
  align-self: center;
  width: 12px;
  height: 12px;
  margin-inline-end: var(--space-2);
  color: var(--color-text-muted);
}

.base-json__leaf {
  display: inline-block;
  padding-inline-start: calc(12px + var(--space-2));
}

.base-json__index {
  color: var(--color-text-muted);
}

.base-json__more {
  margin: 0;
  padding-inline-start: calc(12px + var(--space-2));
  color: var(--color-text-muted);
  font-style: italic;
}
</style>
