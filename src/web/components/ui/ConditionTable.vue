<script lang="tsx">
import { onBeforeUnmount, ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import {
  fieldsForTrigger,
  findField,
  operatorsFor,
  operatorsForPath,
  type EventFieldDefinition,
  type FieldValueKind,
} from '../../../automation/behavior/fields.ts';
import type { EventFilter, FilterOperator } from '../../../automation/behavior/types.ts';
import type { GiftCatalogEntry, ViewerRecord } from '../../../shared/messages.ts';
import { i18nText, t, type Locale } from '../../i18n.ts';
import { FieldIconGlyph, OperatorGlyph, OPERATOR_CODE, OPERATOR_LABELS } from '../condition-icons.vue';
import { GiftPicker, UserPicker } from './GiftPicker.vue';
import { IconSelect } from './IconSelect.vue';
import { normalizeCapturedKey } from './key-capture.ts';
import { Select } from './Select.vue';
import { NumberInput } from './NumberInput.vue';
import { TagsInput } from './TagsInput.vue';
import { TextInput } from './TextInput.vue';
import { InfoTip } from './InfoTip.vue';
import { Tooltip } from './Tooltip.vue';

const COPY = {
  colField: { default: "Field", i18key: "condition.colField" },
  colOperator: { default: "Comparison", i18key: "condition.colOperator" },
  colValue: { default: "Value", i18key: "condition.colValue" },
  add: { default: "Add condition", i18key: "condition.add" },
  remove: { default: "Remove", i18key: "condition.remove" },
  custom: { default: "Another field (advanced)…", i18key: "condition.custom" },
  customPlaceholder: { default: "event.data.whatever", i18key: "condition.customPlaceholder" },
  record: { default: "Record shortcut", i18key: "condition.record" },
  recordHint: { default: "Click, then press the keys to fill this in.", i18key: "condition.recordHint" },
  recording: { default: "Press keys…", i18key: "condition.recording" },
  recordingHint: { default: "Press the keys now. Escape cancels.", i18key: "condition.recordingHint" },
  missing: { default: "value missing", i18key: "condition.missing" },
  missingHint: { default: "With no value this condition never passes: fill it in or remove it.", i18key: "condition.missingHint" },
  choose: { default: "Pick…", i18key: "condition.choose" },
  empty: { default: "No conditions: the event always fires.", i18key: "condition.empty" },
  headHint: { default: "Event data is compared. Every condition must pass; the only \"or\" is the \"is one of\" comparison.", i18key: "condition.headHint" },
  yes: { default: "yes", i18key: "condition.yes" },
  no: { default: "no", i18key: "condition.no" },
  values: { default: "{count} values", i18key: "condition.values" },
} as const;

type ConditionCopy = Omit<{ -readonly [Key in keyof typeof COPY]: string }, 'values'> & { values: (count: number) => string };

function copyFor(locale: Locale): ConditionCopy {
  const copy = {} as Omit<ConditionCopy, 'values'>;
  for (const [key, value] of Object.entries(COPY)) {
    if (key !== 'values') copy[key as keyof Omit<ConditionCopy, 'values'>] = i18nText(locale, value);
  }
  return { ...copy, values: (count) => t(locale, 'condition.values', { count }) };
}

const CUSTOM = '__custom__';

type ConditionTableProps = {
  locale: Locale;
  trigger: string;
  filters: EventFilter[];
  gifts: GiftCatalogEntry[];
  viewers: ViewerRecord[];
  onChange: (filters: EventFilter[]) => void;
};

type PickerState = { index: number; kind: 'gift' | 'user'; multiple: boolean } | null;

/**
 * The compact conditions table: the field comes from a list (never typed), the
 * comparison carries its symbol, and the value uses the editor its type asks
 * for — a gift picker, a viewer picker, a number, a switch.
 */
export const ConditionTable = defineVueComponent<ConditionTableProps>(
  ['locale', 'trigger', 'filters', 'gifts', 'viewers', 'onChange'],
  (props) => {
  const picker = ref<PickerState>(null);
  const recording = ref<{ index: number; path: string } | null>(null);

  const stopRecording = (): void => {
    recording.value = null;
    if (typeof window !== 'undefined') window.removeEventListener('keydown', onRecordKey, true);
  };
  const onRecordKey = (event: KeyboardEvent): void => {
    const target = recording.value;
    if (!target) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === 'Escape') {
      stopRecording();
      return;
    }
    const captured = normalizeCapturedKey(event);
    if (!captured) return;
    const next = /\.modifiers$/.test(target.path) ? captured.modifiers : captured.key;
    update(target.index, { value: next });
    stopRecording();
  };
  const startRecording = (index: number): void => {
    const path = props.filters[index]?.path ?? '';
    stopRecording();
    recording.value = { index, path };
    if (typeof window !== 'undefined') window.addEventListener('keydown', onRecordKey, true);
  };
  onBeforeUnmount(stopRecording);

  const update = (index: number, patch: Partial<EventFilter>): void => {
    props.onChange(props.filters.map((filter, position) => (position === index ? { ...filter, ...patch } : filter)));
  };

  const kindOf = (filter: EventFilter): FieldValueKind => findField(props.trigger, filter.path)?.kind ?? 'text';

  const changeField = (index: number, path: string): void => {
    if (path === CUSTOM) {
      update(index, { path: '', operator: 'eq', value: '', values: undefined });
      return;
    }
    const kind = findField(props.trigger, path)?.kind ?? 'text';
    const operators = operatorsFor(kind);
    const current = props.filters[index]!;
    const operator = operators.includes(current.operator) ? current.operator : operators[0]!;
    update(index, {
      path,
      operator,
      value: operator === 'is-true' || operator === 'is-false' ? '' : current.value,
      values: operator === 'in' ? current.values ?? [] : undefined,
    });
  };

  const addFilter = (): void => {
    const first = fieldsForTrigger(props.trigger)[0];
    const kind = first?.kind ?? 'text';
    props.onChange([
      ...props.filters,
      {
        path: first?.path ?? 'event.user.uniqueId',
        operator: operatorsFor(kind)[0]!,
        value: '',
        values: undefined,
      },
    ]);
  };

  const valuesOf = (filter: EventFilter): string[] => (filter.operator === 'in'
    ? filter.values ?? []
    : filter.value
      ? [filter.value]
      : []);

  const applyPick = (index: number, values: string[]): void => {
    const filter = props.filters[index]!;
    if (filter.operator === 'in') update(index, { values });
    else update(index, { value: values[0] ?? '' });
    picker.value = null;
  };

  return () => {
    const { locale, trigger, filters, gifts, viewers, onChange } = props;
    const copy = copyFor(locale);
    const fields = fieldsForTrigger(trigger);
    return (
    <div class="plg-cond">
      <div class="plg-cond__head">
        <span>
          {copy.colField}
          <InfoTip text={copy.headHint} position="bottom" />
        </span>
        <span>{copy.colOperator}</span>
        <span>{copy.colValue}</span>
        <span />
      </div>

      {filters.length === 0 && <p class="plg-note plg-cond__empty">{copy.empty}</p>}

      {filters.map((filter, index) => {
        const field = findField(trigger, filter.path);
        const kind = kindOf(filter);
        const operators = operatorsForPath(trigger, filter.path);
        const picked = valuesOf(filter);
        const needsValue = filter.operator !== 'is-true' && filter.operator !== 'is-false';
        const missing = needsValue && picked.length === 0;

        return (
          <div class="plg-cond__row" key={`${index}-${filter.path}`}>
            <span class="plg-cond__cell">
              <IconSelect
                class="plg-cond__select"
                ariaLabel={copy.colField}
                value={field ? filter.path : CUSTOM}
                placeholder={copy.custom}
                onChange={(path) => changeField(index, path)}
                options={[
                  ...fields.map((entry) => ({
                    value: entry.path,
                    label: i18nText(locale, entry.label),
                    meta: entry.path,
                    hint: i18nText(locale, entry.hint),
                    icon: <FieldIconGlyph icon={entry.icon} />,
                  })),
                  { value: CUSTOM, label: copy.custom, icon: <FieldIconGlyph icon="text" /> },
                ]}
              />
            </span>

            <span class="plg-cond__cell">
              <IconSelect
                class="plg-cond__select"
                ariaLabel={copy.colOperator}
                value={filter.operator}
                onChange={(next) => {
                  const operator = next as FilterOperator;
                  update(index, {
                    operator,
                    values: operator === 'in' ? filter.values ?? picked : undefined,
                    value: operator === 'in' ? '' : filter.value || picked[0] || '',
                  });
                }}
                options={operators.map((operator) => ({
                  value: operator,
                  label: i18nText(locale, OPERATOR_LABELS[operator]),
                  meta: OPERATOR_CODE[operator],
                  icon: <OperatorGlyph operator={operator} />,
                }))}
              />
            </span>

            <span class="plg-cond__cell plg-cond__cell--value">
              <ConditionValue
                locale={locale}
                filter={filter}
                kind={kind}
                missing={missing}
                copy={copy}
                onOpenPicker={(multiple) => (picker.value = {
                  index,
                  kind: kind === 'gift' ? 'gift' : 'user',
                  multiple,
                })}
                field={field}
                recording={recording.value?.index === index && recording.value?.path === filter.path}
                onRecord={() => startRecording(index)}
                onValue={(value) => update(index, { value })}
                onValues={(values) => update(index, { values })}
              />
            </span>

            <Tooltip text={copy.remove} position="left">
              <button
                type="button"
                class="plg-iconbtn is-danger"
                aria-label={copy.remove}
                onClick={() => onChange(filters.filter((_, position) => position !== index))}
              >
                <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="3 6 5 6 21 6" />
                  <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                </svg>
              </button>
            </Tooltip>

            {!field && (
              <TextInput
                value={filter.path}
                onValueChange={(next) => update(index, { path: next })}
                placeholder={copy.customPlaceholder}
              />
            )}

            {missing && (
              <span class="plg-cond__err">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                  <circle cx="12" cy="12" r="9" />
                  <path d="M12 8v5m0 3h.01" />
                </svg>
                {copy.missingHint}
              </span>
            )}
          </div>
        );
      })}

      <button type="button" class="plg-dashed" onClick={addFilter}>{copy.add}</button>

      {picker.value && picker.value.kind === 'gift' && (
        <GiftPicker
          locale={locale}
          gifts={gifts}
          selected={valuesOf(filters[picker.value.index]!)}
          multiple={picker.value.multiple}
          onPick={(values) => applyPick(picker.value!.index, values)}
          onClose={() => (picker.value = null)}
        />
      )}
      {picker.value && picker.value.kind === 'user' && (
        <UserPicker
          locale={locale}
          viewers={viewers}
          selected={valuesOf(filters[picker.value.index]!)}
          multiple={picker.value.multiple}
          onPick={(values) => applyPick(picker.value!.index, values)}
          onClose={() => (picker.value = null)}
        />
      )}
    </div>
    );
  };
  },
);

type ValueProps = {
  locale: Locale;
  filter: EventFilter;
  kind: FieldValueKind;
  field: EventFieldDefinition | undefined;
  recording: boolean;
  onRecord: () => void;
  missing: boolean;
  copy: ConditionCopy;
  onOpenPicker: (multiple: boolean) => void;
  onValue: (value: string) => void;
  onValues: (values: string[]) => void;
};

/** The value editor the field's type asks for. */
function ConditionValue({ locale, filter, field, recording, onRecord, kind, missing, copy, onOpenPicker, onValue, onValues }: ValueProps) {
  if (filter.operator === 'is-true' || filter.operator === 'is-false') {
    return <span class="plg-cond__fixed">{filter.operator === 'is-true' ? copy.yes : copy.no}</span>;
  }

  if (kind === 'gift' || kind === 'user') {
    const multiple = filter.operator === 'in';
    const values = multiple ? filter.values ?? [] : filter.value ? [filter.value] : [];
    return (
      <button
        type="button"
        class={`plg-cond__pick${missing ? ' is-missing' : ''}`}
        onClick={() => onOpenPicker(multiple)}
      >
        {values.length === 0 && <span>{missing ? copy.missing : copy.choose}</span>}
        {values.length === 1 && <span>{kind === 'user' ? `@${values[0]}` : values[0]}</span>}
        {values.length > 1 && <span>{copy.values(values.length)}</span>}
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="m6 9 6 6 6-6" />
        </svg>
      </button>
    );
  }

  const options = field?.options;
  if (options && options.length > 0 && filter.operator !== 'in') {
    // An empty value counts as filled when it is a listed option ("none").
    const effectivelyMissing = missing && !options.some((option) => option.value === filter.value);
    return (
      <span class="plg-cond__option-value">
        <Select
          value={filter.value}
          onValueChange={onValue}
          options={options.map((option) => ({ value: option.value, label: i18nText(locale, option.label) }))}
          ariaLabel={field ? i18nText(locale, field.label) : copy.colValue}
          error={effectivelyMissing ? copy.missing : undefined}
        />
        <button
          type="button"
          class={`plg-cond__record${recording ? ' is-armed' : ''}`}
          onClick={onRecord}
          aria-pressed={recording}
          aria-label={recording ? copy.recording : copy.record}
          title={recording ? copy.recordingHint : copy.recordHint}
        >
          <span aria-hidden>{recording ? '\u25cf' : '\u2328'}</span>
        </button>
      </span>
    );
  }

  if (filter.operator === 'in') {
    const values = filter.values ?? [];
    return (
      <TagsInput
        value={values}
        onValueChange={onValues}
        placeholder={t(locale, 'condition.addValuePlaceholder')}
        error={missing ? copy.missing : undefined}
        ariaLabel={copy.colValue}
      />
    );
  }

  if (kind === 'number') {
    const parsed = Number(filter.value);
    return (
      <NumberInput
        value={filter.value.trim() === '' || Number.isNaN(parsed) ? null : parsed}
        onValueChange={(next) => onValue(next === null ? '' : String(next))}
        placeholder={missing ? copy.missing : ''}
        error={missing ? copy.missing : undefined}
      />
    );
  }
  return (
    <TextInput
      value={filter.value}
      onValueChange={onValue}
      placeholder={missing ? copy.missing : ''}
      error={missing ? copy.missing : undefined}
    />
  );
}

export default ConditionTable;
</script>
