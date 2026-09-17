<script lang="tsx">
import { nextTick, ref, watch } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../../vue/component.ts';
import { Icon, type IconName } from '../../icons/index.ts';
import type { Locale } from '../../../i18n.ts';
import type { TooltipPosition } from '../tooltip-logic.ts';
import { createNativeSelectEmitter, dispatchControlEvent, normalizeControlString, selectOptionSignature, syncNativeControlValue } from '../control-events.ts';
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
    selectFieldFallback += 1;
    const fallbackId = `tt-select-${selectFieldFallback}`;
    // Native select popups in embedded WebViews can blur (popup close)
    // before `change` fires. Committing on `input` too captures the new
    // value first; the emitter drops the duplicate when `change` follows.
    // No reactive focus state here: the visible ring is painted by
    // `.field__box:focus-within`, and a focus/blur rerender with a stale
    // `props.value` could patch the old selection back before `change`
    // reads the control.
    const selectEmitter = createNativeSelectEmitter((next) => props.onValueChange(next));
    const emitNativeSelection = (event: Event): void => {
      selectEmitter.emit((event.currentTarget as HTMLSelectElement).value);
    };
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
      const normalized = normalizeControlString(value);
      // Parent has acknowledged this value.
      selectEmitter.acknowledge(normalized);
      if (innerRef.value) syncNativeControlValue(innerRef.value, normalized);
    });

    // Dynamic option lists (plugin `optionsFrom`, voices, devices) resolve
    // after first render. The native select keeps the stale empty selection
    // once its options change, so re-apply the value after the new options
    // are in the DOM; otherwise a stored value renders as a blank box.
    //
    // The watch key is a semantic signature (values + disabled state), NOT
    // array identity: SchemaField rebuilds static enum arrays with `.map()`
    // on every render, and an identity watch would rewrite the stale
    // controlled value back into the native select on unrelated rerenders
    // (e.g. reverting `es` to `en`). DOM repair only; never dispatch a fake
    // user selection — the controlled parent already owns `wanted`.
    watch(() => selectOptionSignature(props.options), () => {
      void nextTick().then(() => {
        const control = innerRef.value;
        if (!control) return;
        const wanted = normalizeControlString(props.value);
        const exists = props.options.some(
          (option) => option.value === wanted && !option.disabled,
        );
        if (exists && control.value !== wanted) {
          syncNativeControlValue(control, wanted);
        }
      });
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
              onInput={emitNativeSelection}
              onChange={emitNativeSelection}
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
