<script setup lang="ts">
import { computed, shallowRef, toRaw } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconCode from '@/shared/ui/icon/icons/IconCode.vue';
import IconCollapseAll from '@/shared/ui/icon/icons/IconCollapseAll.vue';
import IconCopy from '@/shared/ui/icon/icons/IconCopy.vue';
import IconExpandAll from '@/shared/ui/icon/icons/IconExpandAll.vue';
import IconTree from '@/shared/ui/icon/icons/IconTree.vue';
import IconWrapText from '@/shared/ui/icon/icons/IconWrapText.vue';
import { buildTree, DEFAULT_TREE_DEPTH, expandableIds, ROOT_ID } from './json-tree.ts';
import { DEFAULT_MAX_CHARS, DEFAULT_MAX_LINES, plainTextLines, prettyJson, tokenizeJson, type JsonLines } from './json-tokens.ts';
import JsonTreeView from './JsonTreeView.vue';
import { useJsonCopy } from './use-json-copy.ts';
import './json.css';

const props = withDefaults(
  defineProps<{
    /** Any JSON-serializable value. Ignored when `source` is given. */
    value?: unknown;
    /** Raw JSON text. Invalid JSON is shown as plain text instead of failing. */
    source?: string;
    /** Accessible name of the scrollable region. Defaults to a generic "JSON document". */
    label?: string;
    lineNumbers?: boolean;
    /** Show the code / tree switch. */
    switchable?: boolean;
    /** Show the copy button. */
    copyable?: boolean;
    /** CSS length (or pixels) after which the area scrolls. */
    maxHeight?: string | number;
    /** Lines kept before the output is cut with a notice. */
    maxLines?: number;
    maxChars?: number;
    /** Print object keys in code order so equal documents look identical. */
    sortKeys?: boolean;
    /** Deepest level the tree can expand; deeper containers stay summaries. */
    treeDepth?: number;
  }>(),
  {
    value: undefined,
    source: undefined,
    label: undefined,
    copyable: true,
    maxHeight: undefined,
    maxLines: DEFAULT_MAX_LINES,
    maxChars: DEFAULT_MAX_CHARS,
    treeDepth: DEFAULT_TREE_DEPTH,
  },
);

const emit = defineEmits<{ copy: [text: string] }>();
const mode = defineModel<'code' | 'tree'>('mode', { default: 'code' });
const wrap = defineModel<boolean>('wrap', { default: true });
const { t } = useI18n();

const parsed = computed<{ ok: true; value: unknown } | { ok: false }>(() => {
  if (props.source === undefined) return { ok: true, value: toRaw(props.value) };
  try { return { ok: true, value: JSON.parse(props.source) as unknown }; } catch { return { ok: false }; }
});
const limits = computed(() => ({ maxLines: props.maxLines, maxChars: props.maxChars, sortKeys: props.sortKeys }));
const shownMode = computed(() => (mode.value === 'tree' && parsed.value.ok ? 'tree' : 'code'));

// Tokenized only while the code view is on screen, and memoized by the computed cache.
const code = computed<JsonLines>(() => {
  if (shownMode.value !== 'code') return { lines: [], truncated: false, plain: false };
  return parsed.value.ok ? tokenizeJson(parsed.value.value, limits.value) : plainTextLines(props.source ?? '', limits.value);
});

const expanded = shallowRef<ReadonlySet<string>>(new Set([ROOT_ID]));
const tree = computed(() => (shownMode.value === 'tree' && parsed.value.ok
  ? buildTree(parsed.value.value, { expanded: expanded.value, maxDepth: props.treeDepth, sortKeys: props.sortKeys })
  : { rows: [], truncated: false }));
const truncated = computed(() => (shownMode.value === 'code' ? code.value.truncated : tree.value.truncated));
const digits = computed(() => String(Math.max(code.value.lines.length, 1)).length);
const maxHeightStyle = computed(() => (props.maxHeight === undefined ? undefined : { '--base-json-max-height': typeof props.maxHeight === 'number' ? `${props.maxHeight}px` : props.maxHeight }));

function toggle(id: string): void {
  const next = new Set(expanded.value);
  if (!next.delete(id)) next.add(id);
  expanded.value = next;
}
function expandAll(): void {
  if (parsed.value.ok) expanded.value = expandableIds(parsed.value.value, { maxDepth: props.treeDepth, sortKeys: props.sortKeys });
}
function collapseAll(): void {
  expanded.value = new Set();
}

const { copied, copy } = useJsonCopy(
  () => parsed.value.ok ? prettyJson(parsed.value.value, { sortKeys: props.sortKeys }) : (props.source ?? ''),
  (text) => emit('copy', text),
);

</script>

<template>
  <div class="base-json">
    <div class="base-json__toolbar">
      <template v-if="switchable && parsed.ok">
        <BaseIconButton
          :label="t('jsonView.code')"
          size="sm"
          :pressed="shownMode === 'code'"
          @click="mode = 'code'"
        >
          <IconCode />
        </BaseIconButton>
        <BaseIconButton
          :label="t('jsonView.tree')"
          size="sm"
          :pressed="shownMode === 'tree'"
          @click="mode = 'tree'"
        >
          <IconTree />
        </BaseIconButton>
        <span class="base-json__divider" />
      </template>
      <template v-if="shownMode === 'tree'">
        <BaseIconButton
          :label="t('jsonView.expandAll')"
          size="sm"
          @click="expandAll"
        >
          <IconExpandAll />
        </BaseIconButton>
        <BaseIconButton
          :label="t('jsonView.collapseAll')"
          size="sm"
          @click="collapseAll"
        >
          <IconCollapseAll />
        </BaseIconButton>
      </template>
      <BaseIconButton
        v-else
        :label="t('jsonView.wrap')"
        size="sm"
        :pressed="wrap"
        @click="wrap = !wrap"
      >
        <IconWrapText />
      </BaseIconButton>
      <BaseIconButton
        v-if="copyable"
        :label="t('jsonView.copy')"
        size="sm"
        @click="copy"
      >
        <IconCheck v-if="copied" />
        <IconCopy v-else />
      </BaseIconButton>
      <span
        class="base-json__status"
        role="status"
      >{{ copied ? t('jsonView.copied') : '' }}</span>
    </div>
    <div
      class="base-json__scroll"
      role="region"
      tabindex="0"
      :aria-label="label ?? t('jsonView.region')"
      :style="maxHeightStyle"
    >
      <pre
        v-if="shownMode === 'code'"
        class="base-json__code"
        :class="{ 'base-json__code--wrap': wrap, 'base-json__code--numbers': lineNumbers }"
        :style="{ '--json-digits': digits }"
      ><span
        v-for="(line, index) in code.lines"
        :key="index"
        class="base-json__line"
        :data-line="index + 1"
      ><span
        v-for="(token, at) in line"
        :key="at"
        :class="`json-token--${token.kind}`"
      >{{ token.text }}</span>{{ index < code.lines.length - 1 ? '\n' : '' }}</span></pre>
      <JsonTreeView
        v-else
        :rows="tree.rows"
        :wrap="wrap"
        @toggle="toggle"
      />
    </div>
    <p
      v-if="truncated"
      class="base-json__notice"
      role="status"
    >
      {{ t('jsonView.truncated') }}
    </p>
  </div>
</template>

<style scoped>
.base-json {
  display: flex;
  flex-direction: column;
  min-width: 0;
  border: var(--border-width) solid var(--color-border);
  border-radius: var(--radius-md);
  background: var(--color-json-bg);
}

.base-json__toolbar {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-1);
  padding: var(--space-1) var(--space-2);
  border-bottom: var(--border-width) solid var(--color-border-subtle);
}

.base-json__divider {
  align-self: stretch;
  width: var(--border-width);
  margin: var(--space-2) var(--space-1);
  background: var(--color-border-subtle);
}

.base-json__status {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
}

.base-json__scroll {
  min-height: 0;
  max-height: var(--base-json-max-height, none);
  overflow: auto;
  outline: none;
}

.base-json__scroll:focus-visible {
  box-shadow: inset 0 0 0 1px var(--color-border-focus);
}

.base-json__code {
  margin: 0;
  padding: var(--space-3) var(--space-4);
  color: var(--color-text);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  line-height: 1.5;
  tab-size: 2;
  white-space: pre;
}

.base-json__code--wrap {
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}

.base-json__code--numbers .base-json__line::before {
  content: attr(data-line);
  display: inline-block;
  min-width: calc(var(--json-digits, 3) * 1ch);
  margin-inline-end: var(--space-5);
  color: var(--color-json-gutter);
  text-align: right;
  user-select: none;
}

.base-json__notice {
  margin: 0;
  padding: var(--space-2) var(--space-4);
  border-top: var(--border-width) solid var(--color-border-subtle);
  color: var(--color-text-muted);
  font-size: var(--font-size-xs);
}
</style>
