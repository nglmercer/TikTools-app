<script setup lang="ts">
import { computed, nextTick, ref, useId } from 'vue';
import { useI18n } from 'vue-i18n';
import BaseEmptyState from '@/shared/ui/empty-state/BaseEmptyState.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconChevronDown from '@/shared/ui/icon/icons/IconChevronDown.vue';
import IconColumns from '@/shared/ui/icon/icons/IconColumns.vue';
import IconEye from '@/shared/ui/icon/icons/IconEye.vue';
import IconEyeOff from '@/shared/ui/icon/icons/IconEyeOff.vue';
import IconFilter from '@/shared/ui/icon/icons/IconFilter.vue';
import IconRows from '@/shared/ui/icon/icons/IconRows.vue';
import IconWarning from '@/shared/ui/icon/icons/IconWarning.vue';
import BaseJsonDiff from '@/shared/ui/json/BaseJsonDiff.vue';
import BaseSegmented from '@/shared/ui/segmented/BaseSegmented.vue';
import { vTooltip } from '@/shared/ui/tooltip/v-tooltip.ts';
import { diffFields, displayLeaf, itemStatus, type ChangeReviewItem, type ChangeReviewStatus } from '../model/change-review.ts';

/**
 * The one change review surface shared by the library review, the automation
 * save conflict review and the sync panel. Two exclusive tabs — elements
 * (semantic field diff) and code (JSON diff) — both with unified or
 * side-by-side layout. Cards stay collapsed by default so the list reads as a
 * git-like change summary; the chevron reveals the details inline. Nothing
 * here opens nested dialogs.
 */
const props = withDefaults(defineProps<{
  items: readonly ChangeReviewItem[];
  /** Shows the profile membership filter and the hidden-leaving indicator. */
  profileFilter?: boolean;
  /** Initial expansion: ids that start open. Everything else starts collapsed. */
  initiallyOpen?: readonly string[];
}>(), { profileFilter: false, initiallyOpen: () => [] });

const emit = defineEmits<{ rename: [id: string, name: string] }>();
const { t } = useI18n();
const uid = useId();

const mode = ref<'elements' | 'code'>('elements');
const layout = ref<'unified' | 'side-by-side'>('unified');
const onlyNew = ref(true);
const showUnchanged = ref(false);
const openIds = ref(new Set(props.initiallyOpen));
// Cards render their details lazily on first expand; once rendered they stay
// mounted so the grid-rows collapse animation always has measurable content.
const renderedIds = ref(new Set(props.initiallyOpen));

const modeOptions = computed(() => [
  { value: 'elements' as const, label: t('changeReview.elements') },
  { value: 'code' as const, label: t('changeReview.code') },
]);

const visibleItems = computed(() => props.items.filter((item) => {
  const status = itemStatus(item);
  if (!showUnchanged.value && status === 'unchanged') return false;
  if (props.profileFilter && onlyNew.value && item.leaving) return false;
  return true;
}));
const statusOrder: Record<ChangeReviewStatus, number> = { added: 0, modified: 1, deleted: 2, unchanged: 3 };
const sortedItems = computed(() => visibleItems.value
  .map((item, index) => ({ item, index }))
  .sort((a, b) => statusOrder[itemStatus(a.item)] - statusOrder[itemStatus(b.item)] || a.index - b.index)
  .map((entry) => entry.item));
const totals = computed(() => {
  const result: Record<ChangeReviewStatus, number> = { added: 0, modified: 0, deleted: 0, unchanged: 0 };
  for (const item of props.items) result[itemStatus(item)] += item.count ?? 1;
  return result;
});
const hiddenLeavingCount = computed(() => {
  if (!props.profileFilter || !onlyNew.value) return 0;
  return props.items
    .filter((item) => item.leaving && (showUnchanged.value || itemStatus(item) !== 'unchanged'))
    .reduce((total, item) => total + (item.count ?? 1), 0);
});

function setOpen(id: string, open: boolean): void {
  const next = new Set(openIds.value);
  if (open) next.add(id);
  else next.delete(id);
  openIds.value = next;
}

/**
 * First expand mounts the details inside the already-collapsed grid track and
 * flips the open class one frame later, so the browser transitions the track
 * from 0fr to 1fr instead of jumping. Collapse only flips the class because
 * the content stays mounted, and the track shrinks progressively.
 */
async function toggle(id: string): Promise<void> {
  if (openIds.value.has(id)) {
    setOpen(id, false);
    return;
  }
  if (renderedIds.value.has(id)) {
    setOpen(id, true);
    return;
  }
  const rendered = new Set(renderedIds.value);
  rendered.add(id);
  renderedIds.value = rendered;
  await nextTick();
  requestAnimationFrame(() => setOpen(id, true));
}
function fieldsOf(item: ChangeReviewItem) {
  return diffFields(item.before, item.after).filter((field) => showUnchanged.value || item.alwaysShowFields || field.status !== 'unchanged');
}
function valueOf(value: unknown): string {
  return displayLeaf(value) || t('changeReview.none');
}
function regionId(id: string): string {
  return `${uid}-${id}`;
}
const symbols: Record<ChangeReviewStatus, string> = { added: '+', modified: '~', deleted: '−', unchanged: '•' };
</script>

<template>
  <div class="change-review">
    <div class="change-review__toolbar">
      <BaseSegmented
        v-model="mode"
        :options="modeOptions"
        size="sm"
        :aria-label="t('changeReview.elements') + ' / ' + t('changeReview.code')"
      />
      <span class="change-review__toolbar-group">
        <BaseIconButton
          :label="t('jsonView.unified')"
          :pressed="layout === 'unified'"
          @click="layout = 'unified'"
        >
          <IconRows />
        </BaseIconButton>
        <BaseIconButton
          :label="t('jsonView.sideBySide')"
          :pressed="layout === 'side-by-side'"
          @click="layout = 'side-by-side'"
        >
          <IconColumns />
        </BaseIconButton>
      </span>
      <span class="change-review__toolbar-group">
        <BaseIconButton
          v-if="profileFilter"
          :label="t('changeReview.onlyNew')"
          :pressed="onlyNew"
          @click="onlyNew = !onlyNew"
        >
          <IconFilter />
        </BaseIconButton>
        <BaseIconButton
          :label="t('changeReview.showUnchanged')"
          :pressed="showUnchanged"
          @click="showUnchanged = !showUnchanged"
        >
          <IconEye v-if="showUnchanged" />
          <IconEyeOff v-else />
        </BaseIconButton>
        <span
          v-if="hiddenLeavingCount"
          v-tooltip="t('changeReview.hiddenLeaving', { count: hiddenLeavingCount })"
          class="change-review__hidden-badge"
          role="status"
          :aria-label="t('changeReview.hiddenLeaving', { count: hiddenLeavingCount })"
        >
          <IconWarning :size="13" />
          {{ hiddenLeavingCount }}
        </span>
      </span>
      <span class="change-review__totals">
        <span
          class="change-review__total change-review__total--added"
          :title="t('changeReview.status.added')"
        >+{{ totals.added }}</span>
        <span
          class="change-review__total change-review__total--modified"
          :title="t('changeReview.status.modified')"
        >~{{ totals.modified }}</span>
        <span
          class="change-review__total change-review__total--deleted"
          :title="t('changeReview.status.deleted')"
        >−{{ totals.deleted }}</span>
      </span>
    </div>

    <BaseEmptyState
      v-if="!sortedItems.length"
      class="change-review__empty"
      :title="t('changeReview.empty')"
      :description="t('changeReview.emptyLead')"
    >
      <template #icon>
        <IconCheck :size="28" />
      </template>
    </BaseEmptyState>

    <div
      v-else
      class="change-review__body"
    >
      <article
        v-for="item in sortedItems"
        :key="item.id"
        class="change-review__card"
        :class="`change-review__card--${itemStatus(item)}`"
      >
        <div class="change-review__card-head">
          <BaseIconButton
            class="change-review__chevron"
            :class="{ 'change-review__chevron--open': openIds.has(item.id) }"
            :label="openIds.has(item.id) ? t('changeReview.collapse') : t('changeReview.expand')"
            :aria-expanded="openIds.has(item.id)"
            :aria-controls="regionId(item.id)"
            @click="toggle(item.id)"
          >
            <IconChevronDown />
          </BaseIconButton>
          <div class="change-review__name">
            <strong v-if="!item.editable">
              {{ item.title }}<span
                v-if="item.count"
                class="change-review__count"
              > ×{{ item.count }}</span>
            </strong>
            <BaseTextField
              v-else
              :model-value="item.title"
              :aria-label="t('changeReview.recordName')"
              data-test="change-review-rename"
              @update:model-value="emit('rename', item.id, $event)"
            />
            <small v-if="item.subtitle">{{ item.subtitle }}</small>
          </div>
          <span
            v-tooltip="t(`changeReview.status.${itemStatus(item)}`)"
            class="change-review__symbol"
            :aria-label="t(`changeReview.status.${itemStatus(item)}`)"
          >{{ symbols[itemStatus(item)] }}</span>
        </div>

        <div
          class="change-review__collapse"
          :class="{ 'change-review__collapse--open': openIds.has(item.id) }"
          :inert="!openIds.has(item.id)"
        >
          <div class="change-review__collapse-inner">
            <div
              v-if="renderedIds.has(item.id)"
              :id="regionId(item.id)"
              class="change-review__details"
              role="region"
            >
          <div
            v-if="mode === 'elements' && layout === 'unified'"
            class="change-review__fields"
          >
            <div
              v-for="field in fieldsOf(item)"
              :key="field.key"
              class="change-review__field"
            >
              <span class="change-review__field-label">{{ field.key }}</span>
              <span class="change-review__field-values">
                <span :class="{ 'change-review__field-old': field.status === 'modified' || field.status === 'deleted' }">{{ valueOf(field.before) }}</span>
                <span class="change-review__arrow">→</span>
                <strong :class="`change-review__field-new change-review__field-new--${field.status}`">{{ valueOf(field.after) }}</strong>
              </span>
            </div>
          </div>

          <div
            v-else-if="mode === 'elements'"
            class="change-review__parallel"
          >
            <div class="change-review__pane change-review__pane--before">
              <span class="change-review__pane-label">{{ t('jsonView.before') }}</span>
              <div
                v-for="field in fieldsOf(item)"
                :key="field.key"
                class="change-review__pane-row"
                :class="{ 'change-review__pane-row--changed': field.status !== 'unchanged' }"
              >
                <span class="change-review__field-label">{{ field.key }}</span>
                <span class="change-review__pane-value">{{ valueOf(field.before) }}</span>
              </div>
            </div>
            <div class="change-review__pane change-review__pane--after">
              <span class="change-review__pane-label">{{ t('jsonView.after') }}</span>
              <div
                v-for="field in fieldsOf(item)"
                :key="field.key"
                class="change-review__pane-row"
                :class="{ 'change-review__pane-row--changed': field.status !== 'unchanged' }"
              >
                <span class="change-review__field-label">{{ field.key }}</span>
                <span class="change-review__pane-value">{{ valueOf(field.after) }}</span>
              </div>
            </div>
          </div>

          <div
            v-else
            class="change-review__code"
          >
            <BaseJsonDiff
              :layout="layout"
              :before="item.before"
              :after="item.after"
              :before-label="t('jsonView.before')"
              :after-label="t('jsonView.after')"
              :label="t('jsonView.diffRegion')"
              :context="showUnchanged ? 12 : 2"
              max-height="360px"
              :switchable="false"
              sort-keys
            />
          </div>
            </div>
          </div>
        </div>
      </article>
    </div>

    <footer
      v-if="$slots.actions"
      class="change-review__footer"
    >
      <slot name="actions" />
    </footer>
  </div>
</template>

<style scoped>
.change-review {
  --review-added: var(--color-text-success);
  --review-modified: var(--color-text-warning);
  --review-deleted: var(--color-text-danger);
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

.change-review__toolbar {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: var(--space-2) var(--space-3);
  padding-bottom: var(--space-2);
}

.change-review__toolbar-group {
  display: inline-flex;
  align-items: center;
  gap: var(--space-1);
}

.change-review__toolbar-group :deep(.base-icon-button[aria-pressed="true"]) {
  border-color: var(--color-border-selected);
  background: color-mix(in srgb, var(--color-accent) 30%, var(--color-bg-control));
  color: var(--color-text-accent);
}

.change-review__hidden-badge {
  display: inline-flex;
  align-items: center;
  gap: 2px;
  color: var(--color-text-warning);
  font-size: var(--font-size-xs);
  cursor: default;
}

.change-review__totals {
  display: flex;
  gap: var(--space-2);
  margin-left: auto;
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
}

.change-review__total {
  padding: 0 var(--space-1);
  border-radius: var(--radius-sm);
  cursor: default;
}

.change-review__total--added { color: var(--review-added); }
.change-review__total--modified { color: var(--review-modified); }
.change-review__total--deleted { color: var(--review-deleted); }

.change-review__body {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-height: 0;
  overflow-y: auto;
}

.change-review__card {
  --item-accent: var(--color-text-muted);
  animation: change-review-card-in var(--duration-normal) ease;
  flex: none;
  padding: var(--space-1) var(--space-2);
  border: var(--border-width) solid var(--color-border-subtle);
  border-left: 3px solid var(--item-accent);
  border-radius: var(--radius-md);
  background: var(--color-bg-panel-header);
}

.change-review__card--added { --item-accent: var(--review-added); }
.change-review__card--modified { --item-accent: var(--review-modified); }
.change-review__card--deleted { --item-accent: var(--review-deleted); }

.change-review__card-head {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-height: 32px;
}

.change-review__chevron {
  margin-left: calc(-1 * var(--space-1));
  transition: transform var(--duration-normal) ease;
}

.change-review__chevron svg {
  transition: transform var(--duration-normal) ease;
}

.change-review__chevron:not(.change-review__chevron--open) svg {
  transform: rotate(-90deg);
}

.change-review__name {
  display: grid;
  flex: 1;
  min-width: 0;
  gap: 1px;
}

.change-review__name strong,
.change-review__name small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.change-review__name small {
  color: var(--color-text-muted);
  font-size: var(--font-size-xs);
}

.change-review__count {
  color: var(--color-text-muted);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-normal);
}

.change-review__symbol {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: color-mix(in srgb, var(--item-accent) 18%, var(--color-bg-control));
  color: var(--item-accent);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-bold);
  cursor: default;
}

.change-review__collapse {
  display: grid;
  grid-template-rows: 0fr;
  transition: grid-template-rows var(--duration-normal) ease;
}

.change-review__collapse--open {
  grid-template-rows: 1fr;
}

.change-review__collapse-inner {
  min-height: 0;
  overflow: hidden;
}

.change-review__details {
  padding: var(--space-1) var(--space-1) var(--space-2) calc(var(--space-2) + 22px);
}

.change-review__fields {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}

.change-review__field {
  display: grid;
  grid-template-columns: minmax(100px, 180px) minmax(0, 1fr);
  gap: var(--space-3);
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
}

.change-review__field:hover {
  background: var(--color-bg-control);
}

.change-review__field-label {
  overflow: hidden;
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.change-review__field-values {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: var(--space-2);
  min-width: 0;
  overflow-wrap: anywhere;
}

.change-review__field-old {
  color: var(--color-text-muted);
  text-decoration: line-through;
}

.change-review__field-new--added { color: var(--review-added); }
.change-review__field-new--modified { color: var(--review-modified); }
.change-review__field-new--deleted { color: var(--review-deleted); }

.change-review__arrow {
  color: var(--color-text-subtle);
}

.change-review__parallel {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-2);
}

.change-review__pane {
  min-width: 0;
  padding: var(--space-2);
  border: var(--border-width) solid var(--color-border-subtle);
  border-radius: var(--radius-sm);
  background: var(--color-bg-control);
}

.change-review__pane--before {
  border-top: 2px solid var(--review-deleted);
}

.change-review__pane--after {
  border-top: 2px solid var(--review-added);
}

.change-review__pane-label {
  display: block;
  margin-bottom: var(--space-1);
  color: var(--color-text-muted);
  font-size: var(--font-size-xs);
  font-weight: var(--font-weight-semibold);
  letter-spacing: .04em;
  text-transform: uppercase;
}

.change-review__pane-row {
  display: grid;
  grid-template-columns: minmax(80px, 130px) minmax(0, 1fr);
  gap: var(--space-2);
  padding: var(--space-1);
  border-radius: var(--radius-sm);
}

.change-review__pane-row--changed {
  background: color-mix(in srgb, var(--review-modified) 8%, transparent);
}

.change-review__pane-value {
  min-width: 0;
  overflow-wrap: anywhere;
  font-size: var(--font-size-sm);
}

.change-review__empty {
  margin: var(--space-4) 0;
}

.change-review__footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: var(--space-2);
  padding-top: var(--space-2);
}

@keyframes change-review-card-in {
  from {
    opacity: 0;
    transform: translateY(3px);
  }

  to {
    opacity: 1;
    transform: none;
  }
}

@media (prefers-reduced-motion: reduce) {
  .change-review__card {
    animation: none;
  }

  .change-review__collapse,
  .change-review__chevron,
  .change-review__chevron svg {
    transition: none;
  }
}

@media (width < 700px) {
  .change-review__field {
    grid-template-columns: 1fr;
    gap: 0;
  }

  .change-review__parallel {
    grid-template-columns: 1fr;
  }

  .change-review__details {
    padding-left: var(--space-1);
  }
}
</style>
