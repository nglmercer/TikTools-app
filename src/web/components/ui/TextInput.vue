<script lang="tsx">
import { ref, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { InfoTip } from './InfoTip.vue';
import { IconClose, IconSearch } from '../icons/index.ts';
import { dispatchControlEvent, normalizeControlString, syncNativeControlValue } from './control-events.ts';

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
  /** MUI-style floating label. When set, the label lives inside until focus/filled. */
  label?: string;
  /** Tooltip-only explanation (ⓘ). Never rendered as a paragraph. */
  hint?: string;
  template?: boolean;
  templateHint?: string;
  prefix?: string;
  suffix?: string;
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
};

export const TextInput = defineVueComponent<TextInputProps>(
  ['value', 'onValueChange', 'placeholder', 'label', 'hint', 'template', 'templateHint', 'prefix', 'suffix', 'disabled', 'readonly', 'error', 'id', 'name', 'type', 'autoComplete', 'spellCheck', 'required', 'clearable', 'onEnter'],
  (props, context) => {
  const innerRef = ref<HTMLInputElement | null>(null);
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
    validate: () => !(props.required && !normalizeControlString(props.value).trim()),
  });

  watch(() => props.value, (value) => {
    if (innerRef.value) syncNativeControlValue(innerRef.value, value);
  });

  const handleInput = (e: Event) => props.onValueChange((e.currentTarget as HTMLInputElement).value);
  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === 'Enter' && props.onEnter) props.onEnter();
  };

  return () => {
    const { placeholder, label, hint, prefix, suffix, disabled, readonly, error, id, name, type = 'text', autoComplete = 'off', spellCheck = false, required, clearable } = props;
    const value = normalizeControlString(props.value);
    if (label) {
      const filled = value.trim().length > 0 || type === 'password' && value.length > 0;
      return (
      <div class={`ui-float ${filled ? 'is-filled' : ''} ${error ? 'has-error' : ''} ${disabled ? 'is-disabled' : ''}`}>
        <div class="ui-float__control">
          {prefix ? <span class="ui-float__prefix">{prefix}</span> : null}
          <input
            ref={innerRef}
            id={id}
            name={name}
            type={type}
            value={value}
            placeholder=" "
            disabled={disabled} readonly={readonly}
            autocomplete={autoComplete}
            spellcheck={spellCheck}
            required={required}
            aria-invalid={Boolean(error)}
            aria-label={label}
            onInput={handleInput}
            onKeydown={handleKeyDown}
          />
          <label class="ui-float__label" for={id}>
            {label}{required ? ' *' : ''}
            {hint ? <InfoTip text={hint} position="right" /> : null}
          </label>
          {clearable && value ? (
            <button type="button" class="ui-float__clear" style={{ right: suffix ? 44 : 8 }} onClick={() => commitProgrammaticValue('')} aria-label="Clear"><IconClose size={10} /></button>
          ) : null}
          {suffix ? <span class="ui-float__suffix">{suffix}</span> : null}
        </div>
        {error ? <span class="ui-float__error">{error}</span> : null}
      </div>
    );
    }

    return (
    <div class={`ui-input ${error ? 'has-error' : ''} ${disabled ? 'is-disabled' : ''} ${prefix ? 'has-prefix' : ''} ${suffix || clearable ? 'has-suffix' : ''}`}>
      {prefix ? <span class="ui-input__prefix">{prefix}</span> : null}
      <input
        ref={innerRef}
        id={id}
        name={name}
        type={type}
        value={value}
        placeholder={placeholder}
        disabled={disabled} readonly={readonly}
        autocomplete={autoComplete}
        spellcheck={spellCheck}
        required={required}
        aria-invalid={Boolean(error)}
        onInput={handleInput}
        onKeydown={handleKeyDown}
      />
      {clearable && value ? (
        <button type="button" class="ui-input__clear" onClick={() => commitProgrammaticValue('')} aria-label="Clear">
          <IconClose size={10} />
        </button>
      ) : null}
      {suffix ? <span class="ui-input__suffix">{suffix}</span> : null}
    </div>
    );
  };
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

export const SearchInput = defineVueComponent<SearchInputProps>(
  ['value', 'onValueChange', 'placeholder', 'disabled', 'id', 'name'],
  (props, context) => {
  const innerRef = ref<HTMLInputElement | null>(null);
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
    validate: () => true,
  });
  watch(() => props.value, (value) => {
    if (innerRef.value) syncNativeControlValue(innerRef.value, value);
  });
  return () => {
    const { placeholder, disabled, id, name } = props;
    const value = normalizeControlString(props.value);
    return (
    <div class={`ui-search ${disabled ? 'is-disabled' : ''}`}>
      <span class="ui-search__icon" aria-hidden="true">
        <IconSearch size={13} />
      </span>
      <input
        ref={innerRef}
        id={id}
        name={name}
        type="text"
        value={value}
        placeholder={placeholder}
        disabled={disabled}
        onInput={(e) => props.onValueChange((e.currentTarget as HTMLInputElement).value)}
      />
      {value ? (
        <button type="button" class="ui-search__clear" onClick={() => commitProgrammaticValue('')} aria-label="Clear search">
          <IconClose size={10} />
        </button>
      ) : null}
    </div>
    );
  };
  },
);

export default TextInput;
</script>
