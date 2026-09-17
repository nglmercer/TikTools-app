<script lang="tsx">
import { ref } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import type { Locale } from '../../i18n.ts';
import { normalizeControlString } from './control-events.ts';
import type { SelectOption } from './controls.ts';
import { SelectField, type SelectFieldHandle } from './fields/index.ts';
import type { FieldSize } from './fields/field-logic.ts';

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
  /** Label-above-input. (Previously a floating label; the prop is unchanged.) */
  label?: string;
  /** Decorative icon before the control. Per-option icons need IconSelect: native <option> cannot draw SVGs. */
  leadingIcon?: VNodeChild;
  size?: FieldSize;
  locale?: Locale;
};

/**
 * Compatibility shim over SelectField: identical props/handles, same shared
 * shell/box/sizing as TextField.
 */
export const Select = defineVueComponent<SelectProps>(
  ['value', 'onValueChange', 'options', 'disabled', 'readonly', 'required', 'ariaLabel', 'error', 'hint', 'id', 'name', 'placeholder', 'label', 'leadingIcon', 'size', 'locale'],
  (props, context) => {
    const fieldRef = ref<SelectFieldHandle | null>(null);
    context.expose({
      getValue: () => fieldRef.value?.getValue() ?? normalizeControlString(props.value),
      setValue: (value: string) => fieldRef.value?.setValue(value),
      focus: () => fieldRef.value?.focus(),
      open: () => fieldRef.value?.open(),
    } satisfies SelectHandle);

    return () => (
      <SelectField
        ref={fieldRef}
        value={normalizeControlString(props.value)}
        onValueChange={props.onValueChange}
        options={props.options}
        disabled={props.disabled}
        readonly={props.readonly}
        required={props.required}
        ariaLabel={props.ariaLabel}
        error={props.error}
        hint={props.hint}
        id={props.id}
        name={props.name}
        placeholder={props.placeholder}
        label={props.label}
        leading={props.leadingIcon}
        size={props.size}
        locale={props.locale}
      />
    );
  },
);

export default Select;
</script>
