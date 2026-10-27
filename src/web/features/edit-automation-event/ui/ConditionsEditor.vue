<script setup lang="ts">
import { computed, onBeforeUnmount, ref, useId } from 'vue';
import type { EventFilter, FilterOperator } from '../../../../automation/behavior/types.ts';
import { normalizeCapturedKey } from '@/entities/automation/lib/key-capture.ts';
import {
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
  withValues,
} from '@/entities/automation/lib/conditions.ts';
import { useAutomationText } from '@/entities/automation/model/use-automation-text.ts';
import BaseButton from '@/shared/ui/button/BaseButton.vue';
import BaseIconButton from '@/shared/ui/button/BaseIconButton.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';
import IconPlus from '@/shared/ui/icon/icons/IconPlus.vue';
import IconRecord from '@/shared/ui/icon/icons/IconRecord.vue';
import IconTrash from '@/shared/ui/icon/icons/IconTrash.vue';
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

const { t, locale, operatorName } = useAutomationText();
const id = useId();
const recording = ref<number | null>(null);

const fields = computed(() => conditionFields(props.trigger, locale.value));
const fieldOptions = computed(() => [
  ...fields.value.map((field) => ({ value: field.path, label: field.label })),
  { value: CUSTOM, label: t('automations.conditions.custom') },
]);

const rows = computed(() =>
  filters.value.map((filter, index) => {
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

function replace(index: number, next: EventFilter): void {
  filters.value = filters.value.map((filter, position) => (position === index ? next : filter));
}

function setField(index: number, path: string | undefined): void {
  if (path === undefined) return;
  replace(index, withField(props.trigger, filters.value[index]!, path === CUSTOM ? null : path));
}

function setOperator(index: number, operator: FilterOperator | undefined): void {
  if (operator) replace(index, withOperator(filters.value[index]!, operator));
}

function setValue(index: number, value: string): void {
  replace(index, { ...filters.value[index]!, value });
}

function setList(index: number, values: string[]): void {
  replace(index, withValues(filters.value[index]!, values));
}

function setPath(index: number, path: string): void {
  replace(index, { ...filters.value[index]!, path });
}

function add(): void {
  filters.value = [...filters.value, newFilter(props.trigger)];
}

function remove(index: number): void {
  filters.value = filters.value.filter((_, position) => position !== index);
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
  const filter = filters.value[index];
  if (filter) setValue(index, /\.modifiers$/.test(filter.path) ? captured.modifiers : captured.key);
  stopRecording();
}

function startRecording(index: number): void {
  stopRecording();
  recording.value = index;
  window.addEventListener('keydown', onRecordKey, true);
}

function stopRecording(): void {
  recording.value = null;
  window.removeEventListener('keydown', onRecordKey, true);
}

onBeforeUnmount(stopRecording);
</script>

<template>
  <div class="conditions">
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
        <span role="columnheader">{{ t('automations.conditions.field') }}</span>
        <span role="columnheader">{{ t('automations.conditions.operator') }}</span>
        <span role="columnheader">{{ t('automations.conditions.value') }}</span>
        <span />
      </div>
      <div
        v-for="row in rows"
        :key="`${row.index}-${row.filter.path}`"
        class="conditions__row"
        role="row"
      >
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
              :label="recording === row.index ? t('automations.conditions.recording') : t('automations.conditions.record')"
              size="sm"
              :pressed="recording === row.index"
              @click="recording === row.index ? stopRecording() : startRecording(row.index)"
            >
              <IconRecord />
            </BaseIconButton>
          </span>
          <BaseTextField
            v-else
            :model-value="row.filter.value"
            size="sm"
            :invalid="row.missing || (row.filter.operator === 'crosses-multiple' && (!Number.isSafeInteger(Number(row.filter.value)) || Number(row.filter.value) <= 0)) || (row.kind === 'number' && row.filter.value.trim() !== '' && Number.isNaN(Number(row.filter.value)))"
            :aria-label="t('automations.conditions.value')"
            :inputmode="row.kind === 'number' ? 'decimal' : undefined"
            @update:model-value="setValue(row.index, $event)"
          />
        </div>
        <BaseIconButton
          :label="t('automations.conditions.remove')"
          size="sm"
          @click="remove(row.index)"
        >
          <IconTrash />
        </BaseIconButton>
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
    </div>
    <p
      v-else
      class="conditions__empty"
    >
      {{ t('automations.conditions.empty') }}
    </p>
    <BaseButton
      class="conditions__add"
      size="sm"
      @click="add"
    >
      <template #icon>
        <IconPlus />
      </template>
      {{ t('automations.conditions.add') }}
    </BaseButton>
  </div>
</template>

<style scoped>
.conditions {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.conditions__grid {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.conditions__head,
.conditions__row {
  display: grid;
  grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr) minmax(0, 1.4fr) auto;
  gap: var(--space-2);
  align-items: center;
}

.conditions__head {
  color: var(--color-text-muted);
  font-size: var(--font-size-sm);
}

.conditions__value {
  min-width: 0;
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
  grid-column: 1 / span 3;
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
