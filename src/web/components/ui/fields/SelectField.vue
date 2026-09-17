<script lang="tsx">
import { ref, watch } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../../vue/component.ts';
import { Icon, type IconName } from '../../icons/index.ts';
import type { Locale } from '../../../i18n.ts';
import type { TooltipPosition } from '../tooltip-logic.ts';
import { dispatchControlEvent, normalizeControlString, syncNativeControlValue } from '../control-events.ts';
import { FieldShell } from './FieldShell.vue';
import { InputGroup } from './InputGroup.vue';
import { describeField, fieldControlId, fieldMessageIds, type FieldSize } from './field-logic.ts';
import type { SelectOption } from '../controls.ts';

export type SelectFieldHandle = {
  getValue: () => string;
  setValue: (v: string) => void;
  focus: () => void;
  open: () => void;
};

export type SelectFieldProps = {
  value: string;
  onValueChange: (v: string) => void;
  options: SelectOption[];
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
  required?: boolean;
  disabled?: boolean;
  readonly?: boolean;
  size?: FieldSize;
  locale?: Locale;
  ariaLabel?: string;
  /** Whitelisted decorative icon before the control. */
  leadingIcon?: IconName;
  /** Raw leading content (compat slot; prefer `leadingIcon`). */
  leading?: VNodeChild;
  className?: string;
};

let selectFieldFallback = 0;

/**
 * Canonical native select: same shell/box/sizing as TextField. Per-option
 * icons need the custom IconSelect — native `<option>` cannot draw SVGs.
 */
export const SelectField = defineVueComponent<SelectFieldProps>(
  ['value', 'onValueChange', 'options', 'id', 'name', 'label', 'hint', 'hintPosition', 'description', 'error', 'placeholder', 'required', 'disabled', 'readonly', 'size', 'locale', 'ariaLabel', 'leadingIcon', 'leading', 'className'],
  (props, context) => {
    const innerRef = ref<HTMLSelectElement | null>(null);
    const focused = ref(false);
    selectFieldFallback += 1;
    const fallbackId = `tt-select-${selectFieldFallback}`;
    const commitProgrammaticValue = (value: string): void => {
      const control = innerRef.value;
      if (control) {
        syncNativeControlValue(control, value);
        dispatchControlEvent(control);
      }
      props.onValueChange(value);
    };
    const openPicker = (): void => {
      const control = innerRef.value as (HTMLSelectElement & { showPicker?: () => void }) | null;
      if (!control || control.disabled) return;
      try {
        if (typeof control.showPicker === 'function') control.showPicker();
        else control.focus();
      } catch {
        control.focus();
      }
    };
    context.expose({
      getValue: () => innerRef.value?.value ?? normalizeControlString(props.value),
      setValue: commitProgrammaticValue,
      focus: () => innerRef.value?.focus(),
      open: openPicker,
    } satisfies SelectFieldHandle);

    watch(() => props.value, (value) => {
      if (innerRef.value) syncNativeControlValue(innerRef.value, value);
    });

    return () => {
      const id = fieldControlId(props, fallbackId);
      const invalid = Boolean(props.error);
      const { descriptionId, errorId } = fieldMessageIds(id);
      const value = normalizeControlString(props.value);
      const isDisabled = props.disabled || props.readonly;
      const leading = props.leading ?? (props.leadingIcon ? <Icon name={props.leadingIcon} size={14} /> : undefined);
      return (
        <FieldShell
          id={id}
          label={props.label}
          hint={props.hint}
          hintPosition={props.hintPosition}
          description={props.description}
          error={props.error}
          required={props.required}
          disabled={isDisabled}
          size={props.size}
          className={props.className}
        >
          <InputGroup
            leading={leading}
            focused={focused.value}
            invalid={invalid}
            disabled={isDisabled}
            trailing={<Icon name="arrow-down" size={12} />}
          >
            <select
              ref={innerRef}
              id={id}
              name={props.name}
              value={value}
              disabled={isDisabled}
              required={props.required}
              aria-invalid={invalid}
              aria-label={props.ariaLabel ?? props.label}
              aria-describedby={describeField([props.description ? descriptionId : undefined, props.error ? errorId : undefined])}
              aria-errormessage={props.error ? errorId : undefined}
              class="field-input field-input--select"
              onChange={(e) => props.onValueChange((e.currentTarget as HTMLSelectElement).value)}
              onFocus={() => { focused.value = true; }}
              onBlur={() => { focused.value = false; }}
            >
              {props.placeholder ? <option value="" disabled>{props.placeholder}</option> : null}
              {props.options.map((o) => (
                <option key={o.value} value={o.value} disabled={o.disabled} title={o.hint ?? o.label}>
                  {o.label}
                </option>
              ))}
            </select>
          </InputGroup>
        </FieldShell>
      );
    };
  },
);

export default SelectField;
</script>
