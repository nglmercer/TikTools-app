<script setup lang="ts">
import { computed, shallowRef, toRaw, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconColumns from '@/shared/ui/icon/icons/IconColumns.vue';
import IconRows from '@/shared/ui/icon/icons/IconRows.vue';
import { diffJson } from './json-diff.ts';
import { visibleDiffRows } from './json-diff-view.ts';
import JsonDiffRows from './JsonDiffRows.vue';
import { DEFAULT_MAX_LINES } from './json-tokens.ts';
import './json.css';

const props = withDefaults(
  defineProps<{
    /** The older version. `undefined` means the document did not exist. */
    before?: unknown;
    /** The newer version. `undefined` means the document no longer exists. */
    after?: unknown;
    /** Heading of the old column in side-by-side layout. */
    beforeLabel?: string;
    afterLabel?: string;
    /** Accessible name of the scrollable region. */
    label?: string;
    /** Show the unified / side-by-side switch. */
    switchable?: boolean;
    /** Unchanged lines kept around each change before a run is folded. */
    context?: number;
    maxHeight?: string | number;
    /** Lines kept per side before the diff covers only the start of the documents. */
    maxLines?: number;
    /** Compare objects with their keys sorted, so reordering is not a change. */
    sortKeys?: boolean;
  }>(),
  { before: undefined, after: undefined, beforeLabel: undefined, afterLabel: undefined, label: undefined, context: 3, maxHeight: undefined, maxLines: DEFAULT_MAX_LINES },
);

const layout = defineModel<'unified' | 'side-by-side'>('layout', { default: 'unified' });
const { t } = useI18n();

const diff = computed(() => diffJson(toRaw(props.before), toRaw(props.after), { maxLines: props.maxLines, sortKeys: props.sortKeys }));
const unfolded = shallowRef<ReadonlySet<number>>(new Set());
watch(diff, () => { unfolded.value = new Set(); });

const items = computed(() => visibleDiffRows(diff.value.rows, props.context, unfolded.value));

const maxHeightStyle = computed(() => (props.maxHeight === undefined ? undefined : { '--base-json-max-height': typeof props.maxHeight === 'number' ? `${props.maxHeight}px` : props.maxHeight }));
const digits = computed(() => {
  let most = 1;
  for (const row of diff.value.rows) most = Math.max(most, row.oldNumber ?? 0, row.newNumber ?? 0);
  return String(most).length;
});

function unfold(start: number): void {
  const next = new Set(unfolded.value);
  next.add(start);
  unfolded.value = next;
}

</script>

<template>
  <div class="base-json-diff">
    <div class="base-json-diff__toolbar">
      <span class="base-json-diff__stats">{{ t('jsonView.stats', { ...diff.stats }) }}</span>
      <template v-if="switchable">
        <BaseIconButton
          :label="t('jsonView.unified')"
          size="sm"
          :pressed="layout === 'unified'"
          @click="layout = 'unified'"
        >
          <IconRows />
        </BaseIconButton>
        <BaseIconButton
          :label="t('jsonView.sideBySide')"
          size="sm"
          :pressed="layout === 'side-by-side'"
          @click="layout = 'side-by-side'"
        >
          <IconColumns />
        </BaseIconButton>
      </template>
    </div>
    <p
      v-if="diff.identical"
      class="base-json-diff__notice"
      role="status"
    >
      {{ t('jsonView.noChanges') }}
    </p>
    <div
      class="base-json-diff__scroll"
      role="region"
      tabindex="0"
      :aria-label="label ?? t('jsonView.diffRegion')"
      :style="[maxHeightStyle, { '--json-digits': digits }]"
    >
      <div
        v-if="layout === 'side-by-side'"
        class="base-json-diff__heads"
      >
        <span>{{ beforeLabel ?? t('jsonView.before') }}</span>
        <span>{{ afterLabel ?? t('jsonView.after') }}</span>
      </div>
      <JsonDiffRows
        :items="items"
        :layout="layout"
        @unfold="unfold"
      />
    </div>
    <p
      v-if="diff.truncated"
      class="base-json-diff__notice"
      role="status"
    >
      {{ t('jsonView.truncated') }}
    </p>
  </div>
</template>

<style scoped>
.base-json-diff {
  display: flex;
  flex-direction: column;
  min-width: 0;
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-json-bg);
}

.base-json-diff__toolbar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-1);
  min-height: calc(var(--control-height-sm) + var(--space-2));
  padding: var(--space-1) var(--space-2);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
}

.base-json-diff__stats {
  flex: 1;
  color: var(--color-text-muted);
  font-size: var(--font-size-xs);
}

.base-json-diff__notice {
  margin: 0;
  padding: var(--space-2) var(--space-4);
  color: var(--color-text-muted);
  font-size: var(--font-size-xs);
}

.base-json-diff__scroll {
  min-height: 0;
  max-height: var(--base-json-max-height, none);
  overflow: auto;
  color: var(--color-text);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  line-height: 1.5;
  outline: none;
}

.base-json-diff__scroll:focus-visible {
  box-shadow: inset 0 0 0 1px var(--color-border-focus);
}

.base-json-diff__heads {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
}

.base-json-diff__heads {
  position: sticky;
  top: 0;
  z-index: 1;
  background: var(--color-bg-panel-header);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
  color: var(--color-text-muted);
  font-family: var(--font-family);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
}

.base-json-diff__heads span {
  padding: var(--space-2) var(--space-4);
}

</style>
