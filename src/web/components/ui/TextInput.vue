<script lang="tsx">
import { ref } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import type { Locale } from '../../i18n.ts';
import { normalizeControlString } from './control-events.ts';
import { SearchField, TextField, type SearchFieldHandle, type TextFieldHandle } from './fields/index.ts';
import type { FieldSize } from './fields/field-logic.ts';

export type TextInputHandle = {
  getValue: () => string;
  setValue: (v: string) => void;
  focus: () => void;
  clear: () => void;
  validate: () => boolean;
};

type TextInputProps = {
  value: string;
  onValueChange: (v: string) => void;
  placeholder?: string;
  /** Label-above-input. (Previously a floating label; the prop is unchanged.) */
  label?: string;
  /** Tooltip-only explanation (ⓘ). Never rendered as a paragraph. */
  hint?: string;
  template?: boolean;
  templateHint?: string;
  prefix?: string;
  suffix?: string;
  /** Decorative icon before the text. Independent from the semantic `prefix`. */
  leadingIcon?: VNodeChild;
  /** Decorative icon after the text. Independent from the semantic `suffix`. */
  trailingIcon?: VNodeChild;
  className?: string;
  disabled?: boolean;
  readonly?: boolean;
  error?: string;
  id?: string;
  name?: string;
  type?: 'text' | 'password';
  autoComplete?: string;
  spellCheck?: boolean;
  required?: boolean;
  clearable?: boolean;
  onEnter?: () => void;
  size?: FieldSize;
  locale?: Locale;
};

/**
 * Compatibility shim over TextField: identical props/handles, shared shell.
 * `label` now renders label-above-input (floating mode removed per spec).
 */
export const TextInput = defineVueComponent<TextInputProps>(
  ['value', 'onValueChange', 'placeholder', 'label', 'hint', 'template', 'templateHint', 'prefix', 'suffix', 'leadingIcon', 'trailingIcon', 'className', 'disabled', 'readonly', 'error', 'id', 'name', 'type', 'autoComplete', 'spellCheck', 'required', 'clearable', 'onEnter', 'size', 'locale'],
  (props, context) => {
    const fieldRef = ref<TextFieldHandle | null>(null);
    context.expose({
      getValue: () => fieldRef.value?.getValue() ?? normalizeControlString(props.value),
      setValue: (value: string) => fieldRef.value?.setValue(value),
      focus: () => fieldRef.value?.focus(),
      clear: () => fieldRef.value?.clear(),
      validate: () => fieldRef.value?.validate() ?? !(props.required && !normalizeControlString(props.value).trim()),
    } satisfies TextInputHandle);

    return () => (
      <TextField
        ref={fieldRef}
        value={normalizeControlString(props.value)}
        onValueChange={props.onValueChange}
        placeholder={props.placeholder}
        label={props.label}
        hint={props.hint}
        prefix={props.prefix}
        suffix={props.suffix}
        leading={props.leadingIcon}
        trailing={props.trailingIcon}
        className={props.className}
        disabled={props.disabled}
        readonly={props.readonly}
        error={props.error}
        id={props.id}
        name={props.name}
        inputType={props.type ?? 'text'}
        autoComplete={props.autoComplete}
        spellCheck={props.spellCheck}
        required={props.required}
        clearable={props.clearable}
        onEnter={props.onEnter}
        size={props.size}
        locale={props.locale}
      />
    );
  },
);

export type SearchInputProps = {
  value: string;
  onValueChange: (v: string) => void;
  placeholder?: string;
  disabled?: boolean;
  id?: string;
  name?: string;
};

/**
 * Pill-shaped search field over SearchField: the magnifier uses the shared
 * leading-icon API and clearing uses the shared clearable control, so there
 * is only one icon/clear implementation to maintain.
 */
export const SearchInput = defineVueComponent<SearchInputProps>(
  ['value', 'onValueChange', 'placeholder', 'disabled', 'id', 'name'],
  (props, context) => {
    const fieldRef = ref<SearchFieldHandle | null>(null);
    context.expose({
      getValue: () => fieldRef.value?.getValue() ?? normalizeControlString(props.value),
      setValue: (value: string) => fieldRef.value?.setValue(value),
      focus: () => fieldRef.value?.focus(),
      clear: () => fieldRef.value?.clear(),
      validate: () => true,
    } satisfies TextInputHandle);
    return () => (
      <SearchField
        ref={fieldRef}
        value={normalizeControlString(props.value)}
        onValueChange={props.onValueChange}
        placeholder={props.placeholder}
        disabled={props.disabled}
        id={props.id}
        name={props.name}
      />
    );
  },
);

export default TextInput;
</script>
