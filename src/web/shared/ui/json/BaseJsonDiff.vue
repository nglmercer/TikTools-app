<script setup lang="ts">
import { computed, shallowRef, toRaw, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconColumns from '@/shared/ui/icon/icons/IconColumns.vue';
import IconExpandAll from '@/shared/ui/icon/icons/IconExpandAll.vue';
import IconRows from '@/shared/ui/icon/icons/IconRows.vue';
import { diffJson, foldRows, segmentLine, type CharRange, type DiffRow } from './json-diff.ts';
import { DEFAULT_MAX_LINES, type JsonLine } from './json-tokens.ts';
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

type Item =
  | { type: 'row'; key: number; row: DiffRow }
  | { type: 'fold'; key: number; count: number };

const items = computed<Item[]>(() => {
  const out: Item[] = [];
  for (const item of foldRows(diff.value.rows, { context: props.context })) {
    if (item.type === 'row') out.push({ type: 'row', key: item.index, row: item.row });
    else if (unfolded.value.has(item.start)) {
      for (let at = item.start; at < item.start + item.count; at += 1) out.push({ type: 'row', key: at, row: diff.value.rows[at]! });
    } else out.push({ type: 'fold', key: item.start, count: item.count });
  }
  return out;
});

interface Line {
  key: string;
  kind: 'unchanged' | 'added' | 'removed';
  number: number | null;
  marker: string;
  tokens: JsonLine;
  ranges: readonly CharRange[];
}
type UnifiedItem = { type: 'fold'; key: number; count: number } | { type: 'line'; key: string; line: Line; oldNumber: number | null; newNumber: number | null };

const sideOld = (row: DiffRow): Line | null => (row.oldTokens === null ? null : {
  key: `${row.oldNumber}`, kind: row.kind === 'unchanged' ? 'unchanged' : 'removed', number: row.oldNumber, marker: row.kind === 'unchanged' ? ' ' : '-', tokens: row.oldTokens, ranges: row.oldRanges,
});
const sideNew = (row: DiffRow): Line | null => (row.newTokens === null ? null : {
  key: `${row.newNumber}`, kind: row.kind === 'unchanged' ? 'unchanged' : 'added', number: row.newNumber, marker: row.kind === 'unchanged' ? ' ' : '+', tokens: row.newTokens, ranges: row.newRanges,
});

const unified = computed<UnifiedItem[]>(() => {
  const out: UnifiedItem[] = [];
  for (const item of items.value) {
    if (item.type === 'fold') { out.push(item); continue; }
    const { row } = item;
    if (row.kind === 'unchanged') {
      out.push({ type: 'line', key: `u${item.key}`, line: sideNew(row)!, oldNumber: row.oldNumber, newNumber: row.newNumber });
      continue;
    }
    const removed = sideOld(row);
    const added = sideNew(row);
    if (removed) out.push({ type: 'line', key: `r${item.key}`, line: removed, oldNumber: row.oldNumber, newNumber: null });
    if (added) out.push({ type: 'line', key: `a${item.key}`, line: added, oldNumber: null, newNumber: row.newNumber });
  }
  return out;
});

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

const markerLabel = (kind: Line['kind']): string => (kind === 'added' ? t('jsonView.lineAdded') : kind === 'removed' ? t('jsonView.lineRemoved') : '');
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
      <template v-if="layout === 'unified'">
        <template
          v-for="item in unified"
          :key="item.key"
        >
          <button
            v-if="item.type === 'fold'"
            class="base-json-diff__fold"
            type="button"
            @click="unfold(item.key)"
          >
            <IconExpandAll />
            <span>{{ t('jsonView.unchanged', { count: item.count }) }}</span>
          </button>
          <div
            v-else
            class="base-json-diff__line base-json-diff__unified"
            :class="`base-json-diff__line--${item.line.kind}`"
          >
            <span
              class="base-json-diff__number"
              aria-hidden="true"
            >{{ item.oldNumber }}</span>
            <span
              class="base-json-diff__number"
              aria-hidden="true"
            >{{ item.newNumber }}</span>
            <span class="base-json-diff__marker"><span aria-hidden="true">{{ item.line.marker }}</span><span
              v-if="item.line.kind !== 'unchanged'"
              class="base-json-diff__sr"
            >{{ markerLabel(item.line.kind) }}</span></span>
            <span class="base-json-diff__text"><span
              v-for="(segment, at) in segmentLine(item.line.tokens, item.line.ranges)"
              :key="at"
              :class="[`json-token--${segment.kind}`, segment.changed && `base-json-diff__word base-json-diff__word--${item.line.kind}`]"
            >{{ segment.text }}</span>{{ '\n' }}</span>
          </div>
        </template>
      </template>
      <template v-else>
        <template
          v-for="item in items"
          :key="item.key"
        >
          <button
            v-if="item.type === 'fold'"
            class="base-json-diff__fold"
            type="button"
            @click="unfold(item.key)"
          >
            <IconExpandAll />
            <span>{{ t('jsonView.unchanged', { count: item.count }) }}</span>
          </button>
          <div
            v-else
            class="base-json-diff__split"
          >
            <template
              v-for="(line, side) in [sideOld(item.row), sideNew(item.row)]"
              :key="side"
            >
              <div
                class="base-json-diff__line base-json-diff__half"
                :class="line ? `base-json-diff__line--${line.kind}` : 'base-json-diff__line--empty'"
              >
                <span
                  class="base-json-diff__number"
                  aria-hidden="true"
                >{{ line?.number }}</span>
                <span class="base-json-diff__marker"><span aria-hidden="true">{{ line?.marker }}</span><span
                  v-if="line && line.kind !== 'unchanged'"
                  class="base-json-diff__sr"
                >{{ markerLabel(line.kind) }}</span></span>
                <span class="base-json-diff__text"><template v-if="line"><span
                  v-for="(segment, at) in segmentLine(line.tokens, line.ranges)"
                  :key="at"
                  :class="[`json-token--${segment.kind}`, segment.changed && `base-json-diff__word base-json-diff__word--${line.kind}`]"
                >{{ segment.text }}</span>{{ '\n' }}</template></span>
              </div>
            </template>
          </div>
        </template>
      </template>
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

.base-json-diff__heads,
.base-json-diff__split {
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

.base-json-diff__half + .base-json-diff__half {
  border-inline-start: var(--border-width) solid var(--color-border-subtle);
}

.base-json-diff__line {
  display: grid;
  min-width: 0;
  align-items: start;
}

.base-json-diff__unified {
  grid-template-columns: calc(var(--json-digits, 3) * 1ch + var(--space-5)) calc(var(--json-digits, 3) * 1ch + var(--space-5)) 2ch minmax(0, 1fr);
}

.base-json-diff__half {
  grid-template-columns: calc(var(--json-digits, 3) * 1ch + var(--space-5)) 2ch minmax(0, 1fr);
}

.base-json-diff__line--added {
  background: var(--color-diff-added-bg);
}

.base-json-diff__line--removed {
  background: var(--color-diff-removed-bg);
}

.base-json-diff__line--empty {
  background: var(--color-bg-panel-header);
}

.base-json-diff__number {
  padding-inline: var(--space-3);
  color: var(--color-json-gutter);
  text-align: right;
  user-select: none;
}

.base-json-diff__marker {
  color: var(--color-text);
  font-weight: var(--font-weight-semibold);
  text-align: center;
  user-select: none;
}

.base-json-diff__sr {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

.base-json-diff__text {
  min-width: 0;
  padding-inline-end: var(--space-4);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.base-json-diff__word {
  border-radius: var(--radius-sm);
  color: var(--color-text);
}

.base-json-diff__word--added {
  background: var(--color-diff-added-word);
}

.base-json-diff__word--removed {
  background: var(--color-diff-removed-word);
}

.base-json-diff__fold {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  width: 100%;
  padding: var(--space-1) var(--space-4);
  border: 0;
  border-block: var(--border-width) solid var(--color-border-subtle);
  background: var(--color-bg-panel-header);
  color: var(--color-text-muted);
  font-family: var(--font-family);
  font-size: var(--font-size-xs);
  text-align: left;
  cursor: pointer;
  outline: none;
  transform: none;
}

.base-json-diff__fold:hover {
  background: var(--color-bg-hover);
  color: var(--color-text);
}

.base-json-diff__fold:focus-visible {
  box-shadow: inset 0 0 0 1px var(--color-border-focus);
}

.base-json-diff__fold :deep(svg) {
  width: 12px;
  height: 12px;
}
</style>
