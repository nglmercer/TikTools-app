<script lang="tsx">
import { ref, watch } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { InfoTip } from './InfoTip.vue';
import { dispatchControlEvent, normalizeControlString, syncNativeControlValue } from './control-events.ts';
import type { SelectOption } from './controls.ts';

export type SelectHandle = {
  getValue: () => string;
  setValue: (v: string) => void;
  focus: () => void;
  open: () => void;
};

export type { SelectOption };

type SelectProps = {
  value: string;
  onValueChange: (v: string) => void;
  options: SelectOption[];
  disabled?: boolean;
  readonly?: boolean;
  required?: boolean;
  ariaLabel?: string;
  error?: string;
  hint?: string;
  id?: string;
  name?: string;
  placeholder?: string;
  /** MUI-style floating label. */
  label?: string;
  /** Decorative icon before the control. Per-option icons need IconSelect: native <option> cannot draw SVGs. */
  leadingIcon?: VNodeChild;
};

export const Select = defineVueComponent<SelectProps>(
  ['value', 'onValueChange', 'options', 'disabled', 'readonly', 'required', 'ariaLabel', 'error', 'hint', 'id', 'name', 'placeholder', 'label', 'leadingIcon'],
  (props, context) => {
  const innerRef = ref<HTMLSelectElement | null>(null);
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
  } satisfies SelectHandle);

  const handleChange = (e: Event) => props.onValueChange((e.currentTarget as HTMLSelectElement).value);
  watch(() => props.value, (value) => {
    if (innerRef.value) syncNativeControlValue(innerRef.value, value);
  });

  return () => {
    const { options, disabled, readonly, required, ariaLabel, error, hint, id, name, placeholder, label, leadingIcon } = props;
    const value = normalizeControlString(props.value);
    const isDisabled = disabled || readonly;
    if (label) {
      const filled = value.trim().length > 0;
      return (
      <div class={`ui-float ${filled ? 'is-filled' : ''} ${error ? 'has-error' : ''} ${isDisabled ? 'is-disabled' : ''}`}>
        <div class={`ui-float__control${leadingIcon ? ' has-leading-icon' : ''}`}>
          {leadingIcon ? <span class="ui-float__leading" aria-hidden="true">{leadingIcon}</span> : null}
          <select ref={innerRef} id={id} name={name} value={value} disabled={isDisabled} required={required} aria-invalid={Boolean(error)} aria-label={ariaLabel ?? label} onChange={handleChange}>
            {placeholder ? (
              <option value="" disabled>
                {placeholder}
              </option>
            ) : null}
            {options.map((o) => (
              <option key={o.value} value={o.value} disabled={o.disabled} title={o.hint ?? o.label}>
                {o.label}
              </option>
            ))}
          </select>
          <label class="ui-float__label" for={id}>
            {label}{required ? ' *' : ''}
            {hint ? <InfoTip text={hint} position="right" /> : null}
          </label>
          <span class="ui-float__arrow" aria-hidden>▾</span>
        </div>
        {error ? <span class="ui-float__error">{error}</span> : null}
      </div>
    );
    }

    return (
    <div class={`ui-select ${error ? 'has-error' : ''} ${isDisabled ? 'is-disabled' : ''} ${leadingIcon ? 'has-leading-icon' : ''}`}>
      {leadingIcon ? <span class="ui-select__leading" aria-hidden="true">{leadingIcon}</span> : null}
      <select ref={innerRef} id={id} name={name} value={value} disabled={isDisabled} required={required} aria-invalid={Boolean(error)} aria-label={ariaLabel} onChange={handleChange}>
        {placeholder ? (
          <option value="" disabled>
            {placeholder}
          </option>
        ) : null}
        {options.map((o) => (
          <option key={o.value} value={o.value} disabled={o.disabled} title={o.hint ?? o.label}>
            {o.label}
          </option>
        ))}
      </select>
      <span class="ui-select__arrow" aria-hidden>
        ▾
      </span>
    </div>
    );
  };
  },
);

export default Select;
</script>
