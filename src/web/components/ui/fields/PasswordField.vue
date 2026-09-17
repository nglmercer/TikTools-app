<script lang="tsx">
import { ref } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../../vue/component.ts';
import { t, type Locale } from '../../../i18n.ts';
import type { TooltipPosition } from '../tooltip-logic.ts';
import { normalizeControlString } from '../control-events.ts';
import { SECRET_PLACEHOLDER } from '../../../../automation/plugins/declarative.ts';
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
  onFocus?: () => void;
  onBlur?: () => void;
  className?: string;
};

/**
 * Password field with an integrated, accessible Show/Hide toggle. The value
 * itself is never logged or rendered anywhere except the native input.
 */
export const PasswordField = defineVueComponent<PasswordFieldProps>(
  ['value', 'onValueChange', 'id', 'name', 'label', 'hint', 'hintPosition', 'description', 'error', 'placeholder', 'required', 'disabled', 'readonly', 'size', 'locale', 'clearable', 'clearLabel', 'showLabel', 'hideLabel', 'autoComplete', 'maxLength', 'onEnter', 'onFocus', 'onBlur', 'className'],
  (props, context) => {
    const fieldRef = ref<TextFieldHandle | null>(null);
    const visible = ref(false);
    // Pure visibility state: toggling must never call onValueChange, setValue,
    // clear, or trigger a save. The controlled `props.value` stays authoritative.
    const toggleVisibility = (): void => {
      visible.value = !visible.value;
    };
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
      // The host never reveals stored secrets: the WebView only ever sees
      // SECRET_PLACEHOLDER ('••••••••'), which looks identical in text and
      // password modes. Toggling it would appear broken, so the toggle stays
      // disabled until the user types a real value. Typing replaces the
      // placeholder instead of appending to it.
      const isStoredPlaceholder = normalizeControlString(props.value) === SECRET_PLACEHOLDER;
      const storedHint = isStoredPlaceholder ? t(locale, 'storedSecretHidden') : undefined;
      const handleValueChange = (next: string): void => {
        const prev = normalizeControlString(props.value);
        if (prev === SECRET_PLACEHOLDER && next !== SECRET_PLACEHOLDER && next.includes(SECRET_PLACEHOLDER)) {
          props.onValueChange(next.replace(SECRET_PLACEHOLDER, ''));
          return;
        }
        props.onValueChange(next);
      };
      const handleFocus = (): void => {
        // Select the placeholder so the first keystroke replaces it cleanly.
        // A plain focus without typing leaves the value untouched (preserved).
        if (normalizeControlString(props.value) === SECRET_PLACEHOLDER) {
          requestAnimationFrame(() => {
            const active = document.activeElement;
            if (active && active instanceof HTMLInputElement) active.select();
          });
        }
        props.onFocus?.();
      };
      const toggle: VNodeChild = (
        <button
          type="button"
          class="field-toggle"
          onMousedown={(event) => {
            // Keep pointer clicks from moving focus away from the password
            // input (which would blur-save the card). Keyboard focus still
            // works: the button remains focusable via Tab + Space/Enter.
            event.preventDefault();
          }}
          onClick={toggleVisibility}
          aria-label={visible.value ? hideLabel : showLabel}
          aria-pressed={visible.value}
          disabled={props.disabled || isStoredPlaceholder}
          title={storedHint}
        >
          {visible.value ? hideLabel : showLabel}
        </button>
      );
      return (
        <TextField
          ref={fieldRef}
          value={props.value}
          onValueChange={handleValueChange}
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
          clearable={props.clearable && !isStoredPlaceholder}
          clearLabel={props.clearLabel}
          inputType={visible.value && !isStoredPlaceholder ? 'text' : 'password'}
          autoComplete={props.autoComplete ?? 'current-password'}
          maxLength={isStoredPlaceholder ? undefined : props.maxLength}
          onEnter={props.onEnter}
          onFocus={handleFocus}
          onBlur={props.onBlur}
          className={props.className}
          trailing={toggle}
        />
      );
    };
  },
);

export default PasswordField;
</script>
