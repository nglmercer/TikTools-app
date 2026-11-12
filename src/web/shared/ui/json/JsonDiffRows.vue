<script setup lang="ts">
import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import IconExpandAll from '@/shared/ui/icon/icons/IconExpandAll.vue';
import { segmentLine } from './json-diff.ts';
import { sideOld, sideNew, unifiedDiffRows, type DiffViewItem, type DiffViewLine } from './json-diff-view.ts';

const props = defineProps<{ items: readonly DiffViewItem[]; layout: 'unified' | 'side-by-side' }>();
const emit = defineEmits<{ unfold: [start: number] }>();
const { t } = useI18n();
const unified = computed(() => unifiedDiffRows(props.items));
const markerLabel = (kind: DiffViewLine['kind']): string => (kind === 'added' ? t('jsonView.lineAdded') : kind === 'removed' ? t('jsonView.lineRemoved') : '');
</script>

<template>
  <template v-if="layout === 'unified'">
    <template
      v-for="item in unified"
      :key="item.key"
    >
      <button
        v-if="item.type === 'fold'"
        class="base-json-diff__fold"
        type="button"
        @click="emit('unfold', item.key)"
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
        @click="emit('unfold', item.key)"
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
</template>

<style scoped>
.base-json-diff__split {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
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
