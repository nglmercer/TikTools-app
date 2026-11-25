<script setup lang="ts">
import { computed, onBeforeUnmount, ref, useId } from 'vue';
import type { ConditionMatchMode, EventFilter, FilterOperator } from '../../../../automation/behavior/types.ts';
import { normalizeCapturedKey } from '@/entities/automation/lib/key-capture.ts';
import {
  captureAlias,
  conditionFields,
  fieldKind,
  filterValues,
  isFilterMissing,
  isKnownField,
  needsValue,
  newFilter,
  OPERATOR_CODE,
  operatorsForFilter,
  withField,
  withOperator,
} from '@/entities/automation/lib/conditions.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import IconCheck from '@/shared/ui/icon/icons/IconCheck.vue';
import IconCopy from '@/shared/ui/icon/icons/IconCopy.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconMoreVertical from '@/shared/ui/icon/icons/IconMoreVertical.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import IconRecord from '@/shared/ui/icon/icons/IconRecord.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
import { fallbackCopy } from '@/shared/ui/json/use-json-copy.ts';
import BaseContextMenu from '@/shared/ui/menu/BaseContextMenu.vue';
import type { MenuAction, MenuEntry } from '@/shared/ui/menu/menu-types.ts';
import BaseSearchSelect from '@/shared/ui/search-select/BaseSearchSelect.vue';
import type { SearchOption } from '@/shared/ui/search-select/search-option.ts';
import BaseTagsInput from '@/shared/ui/tags-input/BaseTagsInput.vue';
import BaseTextField from '@/shared/ui/text-field/BaseTextField.vue';

const CUSTOM = '__custom__';

const props = defineProps<{
  trigger: string;
  gifts: readonly SearchOption[];
  viewers: readonly SearchOption[];
}>();

const filters = defineModel<EventFilter[]>({ required: true });
/** Legacy OR groups from older records; kept in the contract but not edited here. */
const anyOf = defineModel<EventFilter[][]>('anyOf', { default: () => [] });
const matchMode = defineModel<ConditionMatchMode>('matchMode', { default: 'and' });

const { t, locale, operatorName } = useAutomationText();
const id = useId();
const recording = ref<number | null>(null);
/** The footer icon toggles the optional capture view; values stay the default. */
const capturesVisible = ref(false);
const copiedRow = ref<number | null>(null);
let copiedTimer: ReturnType<typeof setTimeout> | undefined;

function captureToken(index: number): string {
  const alias = captureAlias(conditions.value[index]?.as) ?? String(index);
  return `{{ match.${alias} }}`;
}

function conditionLabel(index: number): string {
  const filter = conditions.value[index]!;
  return `${index + 1}. ${filter.path} ${OPERATOR_CODE[filter.operator]} ${filterValues(filter).join(', ')}`;
}

async function copyCapture(index: number): Promise<void> {
  const text = captureToken(index);
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    try { fallbackCopy(text); } catch { return; }
  }
  copiedRow.value = index;
  clearTimeout(copiedTimer);
  copiedTimer = setTimeout(() => { copiedRow.value = null; }, 1600);
}

const fields = computed(() => conditionFields(props.trigger, locale.value));
const modeOptions = computed(() => [
  { value: 'and', label: t('automations.conditions.matchAll') },
  { value: 'or', label: t('automations.conditions.matchAny') },
]);
const fieldOptions = computed(() => [
  ...fields.value.map((field) => ({ value: field.path, label: field.label })),
  { value: CUSTOM, label: t('automations.conditions.custom') },
]);

/** One action list for the kebab, right-click and touch long-press. */
const rowMenu = computed<MenuEntry[]>(() => [
  { id: 'duplicate', label: t('automations.conditions.duplicate'), icon: IconCopy },
  { id: 'remove', label: t('automations.conditions.remove'), icon: IconTrash },
]);

/** A legacy record with only `anyOf` is shown as a flat OR list. */
const conditions = computed(() =>
  matchMode.value === 'or' && filters.value.length === 0 && anyOf.value.length > 0
    ? anyOf.value.flat()
    : filters.value,
);

const rows = computed(() =>
  conditions.value.map((filter, index) => {
    const field = fields.value.find((entry) => entry.path === filter.path);
    const kind = fieldKind(props.trigger, filter.path);
    const options = field?.options ?? [];
    return {
      index,
      filter,
      field,
      kind,
      options,
      known: isKnownField(props.trigger, filter.path),
      operators: operatorsForFilter(props.trigger, filter.path).map((operator) => ({
        value: operator,
        label: `${operatorName(operator)}  ${OPERATOR_CODE[operator]}`,
      })),
      missing: isFilterMissing(filter, options),
    };
  }),
);

/** Only one value per condition; `in` keeps its list because that is its meaning. */
function toSimple(filter: EventFilter): EventFilter {
  const values = filterValues(filter);
  return {
    path: filter.path,
    operator: filter.operator,
    value: values[0] ?? '',
    ...(filter.operator === 'in' ? { values } : {}),
    ...(needsValue(filter.operator) ? { as: captureAlias(filter.as) } : {}),
  };
}

function write(next: readonly EventFilter[]): void {
  filters.value = next.map(toSimple);
  // Clearing a legacy group emits a second patch; only do it when one exists.
  if (anyOf.value.length > 0) anyOf.value = [];
}

function replace(index: number, next: EventFilter): void {
  write(conditions.value.map((filter, position) => (position === index ? next : filter)));
}

function setField(index: number, path: string | undefined): void {
  if (path === undefined) return;
  replace(index, withField(props.trigger, conditions.value[index]!, path === CUSTOM ? null : path));
}

function setOperator(index: number, operator: FilterOperator | undefined): void {
  if (operator) replace(index, withOperator(conditions.value[index]!, operator));
}

function setValue(index: number, value: string): void {
  replace(index, { ...conditions.value[index]!, value });
}

function setList(index: number, values: string[]): void {
  replace(index, { ...conditions.value[index]!, values, value: '' });
}

function setAlias(index: number, alias: string): void {
  replace(index, { ...conditions.value[index]!, as: captureAlias(alias) });
}


function setPath(index: number, path: string): void {
  replace(index, { ...conditions.value[index]!, path });
}

function setMode(value: string | undefined): void {
  if (value !== 'and' && value !== 'or') return;
  const current = conditions.value.map(toSimple);
  matchMode.value = value;
  write(current);
}

function add(): void {
  write([...conditions.value, newFilter(props.trigger)]);
}

function remove(index: number): void {
  write(conditions.value.filter((_, position) => position !== index));
}

function duplicate(index: number): void {
  const source = conditions.value[index]!;
  const copy: EventFilter = { ...toSimple(source), ...(source.values ? { values: [...source.values] } : {}) };
  const list = conditions.value;
  write([...list.slice(0, index + 1), copy, ...list.slice(index + 1)]);
}

/** The kebab feeds the same menu as right-click (desktop) and long-press (mobile). */
function openRowMenu(event: MouseEvent): void {
  const button = event.currentTarget as HTMLElement;
  const rect = button.getBoundingClientRect();
  button.dispatchEvent(
    new MouseEvent('contextmenu', {
      bubbles: true,
      cancelable: true,
      clientX: rect.right,
      clientY: rect.bottom,
    }),
  );
}

function onRowAction(action: MenuAction, index: number): void {
  if (action.id === 'duplicate') duplicate(index);
  else if (action.id === 'remove') remove(index);
}

function pickerFor(kind: string): readonly SearchOption[] {
  return kind === 'gift' ? props.gifts : props.viewers;
}

function onRecordKey(event: KeyboardEvent): void {
  const index = recording.value;
  if (index === null) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === 'Escape') {
    stopRecording();
    return;
  }
  const captured = normalizeCapturedKey(event);
  if (!captured) return;
  const filter = conditions.value[index];
  if (filter) setValue(index, /\.modifiers$/.test(filter.path) ? captured.modifiers : captured.key);
  stopRecording();
}

function startRecording(index: number): void {
  stopRecording();
  recording.value = index;
  window.addEventListener('keydown', onRecordKey, true);
}

function isRecording(index: number): boolean {
  return recording.value === index;
}

function stopRecording(): void {
  recording.value = null;
  window.removeEventListener('keydown', onRecordKey, true);
}

onBeforeUnmount(() => {
  stopRecording();
  clearTimeout(copiedTimer);
});
</script>

<template>
  <div
    v-if="!capturesVisible"
    class="conditions"
  >
    <div
      v-if="rows.length"
      class="conditions__grid"
      role="table"
      :aria-label="t('automations.event.conditions')"
    >
      <div
        class="conditions__head"
        role="row"
      >
        <span />
        <span role="columnheader">{{ t('automations.conditions.field') }}</span>
        <span role="columnheader">{{ t('automations.conditions.operator') }}</span>
        <span role="columnheader">{{ t('automations.conditions.value') }}</span>
      </div>
      <BaseContextMenu
        v-for="row in rows"
        :key="`${row.index}-${row.filter.path}`"
        :items="rowMenu"
        @select="onRowAction($event, row.index)"
      >
        <div
          class="conditions__row"
          role="row"
        >
          <BaseIconButton
            :label="t('automations.conditions.actions')"
            size="sm"
            aria-haspopup="menu"
            @click="openRowMenu($event)"
          >
            <IconMoreVertical />
          </BaseIconButton>
          <BaseComboBox
            :model-value="row.known ? row.filter.path : CUSTOM"
            :options="fieldOptions"
            size="sm"
            :aria-label="t('automations.conditions.field')"
            @update:model-value="setField(row.index, $event)"
          />
          <BaseComboBox
            :model-value="row.filter.operator"
            :options="row.operators"
            size="sm"
            :aria-label="t('automations.conditions.operator')"
            @update:model-value="setOperator(row.index, $event)"
          />
          <div class="conditions__value">
            <span
              v-if="!needsValue(row.filter.operator)"
              class="conditions__fixed"
            >{{ row.filter.operator === 'is-true' ? t('automations.conditions.yes') : t('automations.conditions.no') }}</span>
            <BaseTagsInput
              v-else-if="row.filter.operator === 'in'"
              :model-value="filterValues(row.filter)"
              :placeholder="t('automations.conditions.addValue')"
              :aria-label="t('automations.conditions.value')"
              @update:model-value="setList(row.index, $event)"
            />
            <BaseSearchSelect
              v-else-if="row.kind === 'gift' || row.kind === 'user'"
              :model-value="row.filter.value"
              :options="pickerFor(row.kind)"
              :placeholder="row.kind === 'gift' ? t('automations.pickers.giftSearch') : t('automations.pickers.viewerSearch')"
              allow-custom
              :invalid="row.missing"
              :aria-label="t('automations.conditions.value')"
              @update:model-value="setValue(row.index, $event)"
            />
            <span
              v-else-if="row.options.length"
              class="conditions__option"
            >
              <BaseComboBox
                :model-value="row.filter.value"
                :options="row.options"
                size="sm"
                :aria-label="t('automations.conditions.value')"
                @update:model-value="setValue(row.index, String($event ?? ''))"
              />
              <BaseIconButton
                :label="isRecording(row.index) ? t('automations.conditions.recording') : t('automations.conditions.record')"
                size="sm"
                :pressed="isRecording(row.index)"
                @click="isRecording(row.index) ? stopRecording() : startRecording(row.index)"
              >
                <IconRecord />
              </BaseIconButton>
            </span>
            <BaseTextField
              v-else
              :model-value="row.filter.value"
              size="sm"
              :invalid="row.missing || (row.kind === 'number' && row.filter.value.trim() !== '' && Number.isNaN(Number(row.filter.value)))"
              :aria-label="t('automations.conditions.value')"
              :inputmode="row.kind === 'number' ? 'decimal' : undefined"
              @update:model-value="setValue(row.index, $event)"
            />
          </div>
          <BaseTextField
            v-if="!row.known"
            :id="`${id}-path-${row.index}`"
            class="conditions__path"
            :model-value="row.filter.path"
            size="sm"
            :placeholder="t('automations.conditions.customPlaceholder')"
            :aria-label="t('automations.conditions.customPath')"
            spellcheck="false"
            @update:model-value="setPath(row.index, $event)"
          />
          <p
            v-if="row.missing"
            class="conditions__missing"
            role="status"
          >
            {{ t('automations.conditions.missing') }}
          </p>
        </div>
      </BaseContextMenu>
    </div>
    <p
      v-else
      class="conditions__empty"
    >
      {{ t('automations.conditions.empty') }}
    </p>
  </div>
  <div
    v-else
    class="conditions__captures"
  >
    <div
      v-for="row in rows.filter((entry) => needsValue(entry.filter.operator))"
      :key="row.index"
      class="conditions__capture"
    >
      <span
        class="conditions__capturekey"
        :title="t('automations.conditions.capturesHint', { token: captureToken(row.index) })"
      >{{ conditionLabel(row.index) }}</span>
      <BaseTextField
        :model-value="row.filter.as ?? ''"
        size="sm"
        :placeholder="t('automations.conditions.aliasPlaceholder')"
        :aria-label="t('automations.conditions.alias')"
        spellcheck="false"
        @update:model-value="setAlias(row.index, $event)"
      />
      <BaseIconButton
        :label="copiedRow === row.index ? t('common.copied') : t('common.copy')"
        size="sm"
        @click="copyCapture(row.index)"
      >
        <IconCheck v-if="copiedRow === row.index" />
        <IconCopy v-else />
      </BaseIconButton>
    </div>
  </div>
  <div class="conditions__foot">
    <BaseIconButton
      class="conditions__add"
      :label="t('automations.conditions.add')"
      variant="default"
      size="sm"
      @click="add"
    >
      <IconPlus />
    </BaseIconButton>
    <BaseIconButton
      :label="t('automations.conditions.captures')"
      size="sm"
      :pressed="capturesVisible"
      @click="capturesVisible = !capturesVisible"
    >
      <IconFlow />
    </BaseIconButton>
    <div
      v-if="rows.length > 1"
      class="conditions__mode"
    >
      <span class="conditions__modelabel">{{ t('automations.conditions.matchMode') }}</span>
      <BaseComboBox
        :model-value="matchMode"
        :options="modeOptions"
        size="sm"
        :aria-label="t('automations.conditions.matchMode')"
        @update:model-value="setMode"
      />
    </div>
  </div>
</template>

<style scoped>
.conditions {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.conditions__foot {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.conditions__mode {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.conditions__modelabel {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
  font-weight: var(--font-weight-semibold);
}

.conditions__grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.conditions__head,
.conditions__row {
  display: grid;
  grid-template-columns: auto minmax(0, 1.3fr) minmax(0, 1fr) minmax(0, 1.4fr);
  gap: var(--space-2);
  align-items: center;
}

/* Every control carries its own label, so the column titles are only kept for assistive tech. */
.conditions__head {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}

.conditions__value {
  min-width: 0;
}

.conditions__captures {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.conditions__capture {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.conditions__capture > .base-text-field {
  flex: none;
  width: 7rem;
}

.conditions__capturekey {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  color: var(--color-text-muted);
  font-family: var(--font-family-mono);
  font-size: var(--font-size-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.conditions__option {
  display: flex;
  align-items: center;
  gap: var(--space-1);
}

.conditions__option > :first-child {
  flex: 1;
  min-width: 0;
}

.conditions__fixed {
  color: var(--color-text-muted);
}

.conditions__path {
  grid-column: 2 / -1;
}

.conditions__missing {
  grid-column: 1 / -1;
  margin: 0;
  color: var(--color-warning);
  font-size: var(--font-size-sm);
}

.conditions__empty {
  margin: 0;
  color: var(--color-text-subtle);
}

.conditions__add {
  align-self: flex-start;
}
</style>
