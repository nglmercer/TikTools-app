<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import type { Locale } from '../../i18n.ts';
import { normalizeControlString } from './control-events.ts';
import { PasswordField, type PasswordFieldHandle } from './fields/index.ts';
import type { FieldSize } from './fields/field-logic.ts';

export type PasswordInputHandle = {
  getValue: () => string;
  setValue: (v: string) => void;
  focus: () => void;
  clear: () => void;
};

type PasswordInputProps = {
  value: string;
  onValueChange: (v: string) => void;
  label?: string;
  hint?: string;
  placeholder?: string;
  disabled?: boolean;
  required?: boolean;
  error?: string;
  id?: string;
  name?: string;
  autoComplete?: string;
  clearable?: boolean;
  size?: FieldSize;
  locale?: Locale;
};

/**
 * Compatibility shim over PasswordField: identical props/handles, shared
 * shell with label-above-input and integrated Show/Hide toggle.
 */
export const PasswordInput = defineVueComponent<PasswordInputProps>(
  ['value', 'onValueChange', 'label', 'hint', 'placeholder', 'disabled', 'required', 'error', 'id', 'name', 'autoComplete', 'clearable', 'size', 'locale'],
  (props, context) => {
    const fieldRef = ref<PasswordFieldHandle | null>(null);
    context.expose({
      getValue: () => fieldRef.value?.getValue() ?? normalizeControlString(props.value),
      setValue: (value: string) => fieldRef.value?.setValue(value),
      focus: () => fieldRef.value?.focus(),
      clear: () => fieldRef.value?.clear(),
    } satisfies PasswordInputHandle);

    return () => (
      <PasswordField
        ref={fieldRef}
        value={normalizeControlString(props.value)}
        onValueChange={props.onValueChange}
        label={props.label}
        hint={props.hint}
        placeholder={props.placeholder}
        disabled={props.disabled}
        required={props.required}
        error={props.error}
        id={props.id}
        name={props.name}
        autoComplete={props.autoComplete}
        clearable={props.clearable}
        size={props.size}
        locale={props.locale}
      />
    );
  },
);

export default PasswordInput;
</script>
