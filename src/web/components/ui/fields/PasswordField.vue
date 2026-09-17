<script lang="tsx">
import { ref } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../../vue/component.ts';
import { t, type Locale } from '../../../i18n.ts';
import type { TooltipPosition } from '../tooltip-logic.ts';
import { normalizeControlString } from '../control-events.ts';
import { TextField, type TextFieldHandle } from './TextField.vue';
import type { FieldSize } from './field-logic.ts';

export type PasswordFieldHandle = {
  getValue: () => string;
  setValue: (v: string) => void;
  focus: () => void;
  clear: () => void;
};

export type PasswordFieldProps = {
  value: string;
  onValueChange: (v: string) => void;
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
  clearable?: boolean;
  clearLabel?: string;
  showLabel?: string;
  hideLabel?: string;
  autoComplete?: string;
  maxLength?: number;
  onEnter?: () => void;
  className?: string;
};

/**
 * Password field with an integrated, accessible Show/Hide toggle. The value
 * itself is never logged or rendered anywhere except the native input.
 */
export const PasswordField = defineVueComponent<PasswordFieldProps>(
  ['value', 'onValueChange', 'id', 'name', 'label', 'hint', 'hintPosition', 'description', 'error', 'placeholder', 'required', 'disabled', 'readonly', 'size', 'locale', 'clearable', 'clearLabel', 'showLabel', 'hideLabel', 'autoComplete', 'maxLength', 'onEnter', 'className'],
  (props, context) => {
    const fieldRef = ref<TextFieldHandle | null>(null);
    const visible = ref(false);
    context.expose({
      getValue: () => fieldRef.value?.getValue() ?? normalizeControlString(props.value),
      setValue: (value: string) => fieldRef.value?.setValue(value),
      focus: () => fieldRef.value?.focus(),
      clear: () => fieldRef.value?.clear(),
    } satisfies PasswordFieldHandle);

    return () => {
      const locale: Locale = props.locale ?? 'en';
      const showLabel = props.showLabel ?? t(locale, 'showPassword');
      const hideLabel = props.hideLabel ?? t(locale, 'hidePassword');
      const toggle: VNodeChild = (
        <button
          type="button"
          class="field-toggle"
          onClick={() => { visible.value = !visible.value; }}
          aria-label={visible.value ? hideLabel : showLabel}
          aria-pressed={visible.value}
          disabled={props.disabled}
        >
          {visible.value ? hideLabel : showLabel}
        </button>
      );
      return (
        <TextField
          ref={fieldRef}
          value={props.value}
          onValueChange={props.onValueChange}
          id={props.id}
          name={props.name}
          label={props.label}
          hint={props.hint}
          hintPosition={props.hintPosition}
          description={props.description}
          error={props.error}
          placeholder={props.placeholder}
          required={props.required}
          disabled={props.disabled}
          readonly={props.readonly}
          size={props.size}
          locale={props.locale}
          clearable={props.clearable}
          clearLabel={props.clearLabel}
          inputType={visible.value ? 'text' : 'password'}
          autoComplete={props.autoComplete ?? 'current-password'}
          maxLength={props.maxLength}
          onEnter={props.onEnter}
          className={props.className}
          trailing={toggle}
        />
      );
    };
  },
);

export default PasswordField;
</script>
