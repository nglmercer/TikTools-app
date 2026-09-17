<script lang="tsx">
import { ref, watch } from 'vue';
import { defineVueComponent } from '../../../vue/component.ts';
import { Icon } from '../../icons/index.ts';
import { t, type Locale } from '../../../i18n.ts';
import type { TooltipPosition } from '../tooltip-logic.ts';
import { dispatchControlEvent, syncNativeControlValue } from '../control-events.ts';
import { clampNumber } from '../controls.ts';
import { FieldShell } from './FieldShell.vue';
import { InputGroup } from './InputGroup.vue';
import { describeField, fieldControlId, fieldMessageIds, parseNumberDraft, roundToStep, type FieldSize } from './field-logic.ts';

export type NumberFieldHandle = {
  getValue: () => number | null;
  setValue: (v: number | null) => void;
  focus: () => void;
};

export type NumberFieldProps = {
  value: number | null;
  onValueChange: (v: number | null) => void;
  id?: string;
  name?: string;
  label?: string;
  /** Tooltip-only explanation (ⓘ). Never rendered as a paragraph. */
  hint?: string;
  hintPosition?: TooltipPosition;
  /** Visible help under the control. Hidden while an error shows. */
  description?: string;
  error?: string;
  placeholder?: string;
  min?: number;
  max?: number;
  step?: number;
  required?: boolean;
  disabled?: boolean;
  readonly?: boolean;
  size?: FieldSize;
  locale?: Locale;
  /** Semantic text pinned after the input (`ms`, `%`). Not an icon. */
  suffix?: string;
  increaseLabel?: string;
  decreaseLabel?: string;
  className?: string;
};

let numberFieldFallback = 0;

/**
 * Canonical number field: same shell/sizing as TextField, draft-preserving
 * typing (intermediate `-`, `1.` stay editable), clamp-on-commit steppers.
 */
export const NumberField = defineVueComponent<NumberFieldProps>(
  ['value', 'onValueChange', 'id', 'name', 'label', 'hint', 'hintPosition', 'description', 'error', 'placeholder', 'min', 'max', 'step', 'required', 'disabled', 'readonly', 'size', 'locale', 'suffix', 'increaseLabel', 'decreaseLabel', 'className'],
  (props, context) => {
    const innerRef = ref<HTMLInputElement | null>(null);
    const focused = ref(false);
    // Draft keeps intermediate typing states (empty, "-", "1.") possible.
    const draft = ref<string | null>(null);
    numberFieldFallback += 1;
    const fallbackId = `tt-number-${numberFieldFallback}`;
    const display = (): string => {
      if (draft.value !== null) return draft.value;
      return props.value === null || props.value === undefined ? '' : String(props.value);
    };
    const commitProgrammaticValue = (value: number | null): void => {
      draft.value = null;
      const next = value === null ? null : clampNumber(roundToStep(value, props.step), props.min, props.max);
      const control = innerRef.value;
      if (control) {
        syncNativeControlValue(control, next === null ? '' : String(next));
        dispatchControlEvent(control);
      }
      props.onValueChange(next);
    };
    context.expose({
      getValue: () => props.value,
      setValue: commitProgrammaticValue,
      focus: () => innerRef.value?.focus(),
    } satisfies NumberFieldHandle);

    watch(() => props.value, (value) => {
      draft.value = null;
      if (innerRef.value) syncNativeControlValue(innerRef.value, value === null || value === undefined ? '' : String(value));
    });

    const parseAndEmit = (raw: string): void => {
      const parsed = parseNumberDraft(raw);
      if (parsed.kind === 'empty') {
        draft.value = raw;
        props.onValueChange(null);
        return;
      }
      if (parsed.kind === 'invalid') {
        draft.value = raw;
        return;
      }
      // While typing, don't clamp (would block intermediate values).
      draft.value = raw;
      props.onValueChange(roundToStep(parsed.value, props.step));
    };

    const commitClamp = (): void => {
      if (draft.value === null) return;
      const raw = draft.value;
      draft.value = null;
      const parsed = parseNumberDraft(raw);
      if (parsed.kind === 'empty') {
        props.onValueChange(null);
        return;
      }
      if (parsed.kind === 'invalid') return;
      props.onValueChange(clampNumber(roundToStep(parsed.value, props.step), props.min, props.max));
    };

    const nudge = (dir: 1 | -1): void => {
      if (props.disabled || props.readonly) return;
      const base = props.value ?? 0;
      commitProgrammaticValue(base + dir * (props.step ?? 1));
    };

    return () => {
      const id = fieldControlId(props, fallbackId);
      const invalid = Boolean(props.error);
      const { descriptionId, errorId } = fieldMessageIds(id);
      const locale: Locale = props.locale ?? 'en';
      return (
        <FieldShell
          id={id}
          label={props.label}
          hint={props.hint}
          hintPosition={props.hintPosition}
          description={props.description}
          error={props.error}
          required={props.required}
          disabled={props.disabled}
          size={props.size}
          className={props.className}
        >
          <InputGroup
            suffix={props.suffix}
            focused={focused.value}
            invalid={invalid}
            disabled={props.disabled}
            trailing={(
              <span class="field-steppers">
                <button
                  type="button"
                  class="field-stepper field-stepper--up"
                  onClick={() => nudge(1)}
                  disabled={props.disabled || props.readonly}
                  aria-label={props.increaseLabel ?? t(locale, 'increaseValue')}
                >
                  <Icon name="arrow-down" size={10} />
                </button>
                <button
                  type="button"
                  class="field-stepper"
                  onClick={() => nudge(-1)}
                  disabled={props.disabled || props.readonly}
                  aria-label={props.decreaseLabel ?? t(locale, 'decreaseValue')}
                >
                  <Icon name="arrow-down" size={10} />
                </button>
              </span>
            )}
          >
            <input
              ref={innerRef}
              id={id}
              name={props.name}
              type="number"
              value={display()}
              min={props.min}
              max={props.max}
              step={props.step ?? 1}
              placeholder={props.placeholder}
              disabled={props.disabled}
              readonly={props.readonly}
              required={props.required}
              aria-invalid={invalid}
              aria-describedby={describeField([props.description ? descriptionId : undefined, props.error ? errorId : undefined])}
              aria-errormessage={props.error ? errorId : undefined}
              class="field-input field-input--number"
              onInput={(e) => parseAndEmit((e.currentTarget as HTMLInputElement).value)}
              onChange={commitClamp}
              onFocus={() => { focused.value = true; }}
              onBlur={() => { focused.value = false; commitClamp(); }}
            />
          </InputGroup>
        </FieldShell>
      );
    };
  },
);

export default NumberField;
</script>
