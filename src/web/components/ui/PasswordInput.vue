<script lang="tsx">
import { ref, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { FormField } from './FormField.vue';
import { IconClose } from '../icons/index.ts';
import { dispatchControlEvent, normalizeControlString, syncNativeControlValue } from './control-events.ts';
import { fieldIds } from './controls.ts';

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
};

let passwordFallback = 0;

export const PasswordInput = defineVueComponent<PasswordInputProps>(
  ['value', 'onValueChange', 'label', 'hint', 'placeholder', 'disabled', 'required', 'error', 'id', 'name', 'autoComplete', 'clearable'],
  (props, context) => {
  const innerRef = ref<HTMLInputElement | null>(null);
  const visible = ref(false);
  const commitProgrammaticValue = (value: string): void => {
    const control = innerRef.value;
    if (control) {
      syncNativeControlValue(control, value);
      dispatchControlEvent(control);
    }
    props.onValueChange(value);
  };
  context.expose({
    getValue: () => innerRef.value?.value ?? normalizeControlString(props.value),
    setValue: commitProgrammaticValue,
    focus: () => innerRef.value?.focus(),
    clear: () => commitProgrammaticValue(''),
  } satisfies PasswordInputHandle);
  watch(() => props.value, (value) => {
    if (innerRef.value) syncNativeControlValue(innerRef.value, value);
  });
  return () => {
    passwordFallback += 1;
    const { id, describedBy } = fieldIds(props, `tt-password-${passwordFallback}`);
    const value = normalizeControlString(props.value);
    const hintId = props.hint ? `${id}-hint` : undefined;
    const errorId = props.error ? `${id}-error` : undefined;
    const control = (
      <div class={`ui-input ui-password ${props.error ? 'has-error' : ''} ${props.disabled ? 'is-disabled' : ''}`}>
        <input
          ref={innerRef}
          id={id}
          name={props.name}
          type={visible.value ? 'text' : 'password'}
          value={value}
          placeholder={props.placeholder}
          disabled={props.disabled}
          required={props.required}
          autocomplete={props.autoComplete ?? 'current-password'}
          aria-invalid={Boolean(props.error)}
          aria-describedby={describedBy([hintId, errorId])}
          onInput={(e) => props.onValueChange((e.currentTarget as HTMLInputElement).value)}
        />
        {props.clearable && value ? (
          <button type="button" class="ui-input__clear" onClick={() => commitProgrammaticValue('')} aria-label="Clear password"><IconClose size={10} /></button>
        ) : null}
        <button
          type="button"
          class="ui-password__toggle"
          onClick={() => { visible.value = !visible.value; }}
          aria-label={visible.value ? 'Hide password' : 'Show password'}
          aria-pressed={visible.value}
          disabled={props.disabled}
        >
          {visible.value ? 'Hide' : 'Show'}
        </button>
      </div>
    );
    if (!props.label && !props.hint && !props.error) return control;
    return (
      <FormField label={props.label} hint={props.hint} error={props.error} htmlFor={id} required={props.required}>
        {control}
      </FormField>
    );
  };
  },
);

export default PasswordInput;
</script>
