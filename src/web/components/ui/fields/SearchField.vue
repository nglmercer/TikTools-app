<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../../vue/component.ts';
import type { Locale } from '../../../i18n.ts';
import type { TooltipPosition } from '../tooltip-logic.ts';
import { normalizeControlString } from '../control-events.ts';
import { TextField, type TextFieldHandle } from './TextField.vue';
import type { FieldSize } from './field-logic.ts';

export type SearchFieldHandle = {
  getValue: () => string;
  setValue: (v: string) => void;
  focus: () => void;
  clear: () => void;
  validate: () => boolean;
};

export type SearchFieldProps = {
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
  disabled?: boolean;
  readonly?: boolean;
  size?: FieldSize;
  locale?: Locale;
  clearLabel?: string;
  onEnter?: () => void;
  className?: string;
};

/**
 * Pill-shaped search field on TextField/InputGroup: `search` leading icon,
 * native `search` input, shared clear control. No second icon/clear
 * implementation.
 */
export const SearchField = defineVueComponent<SearchFieldProps>(
  ['value', 'onValueChange', 'id', 'name', 'label', 'hint', 'hintPosition', 'description', 'error', 'placeholder', 'disabled', 'readonly', 'size', 'locale', 'clearLabel', 'onEnter', 'className'],
  (props, context) => {
    const fieldRef = ref<TextFieldHandle | null>(null);
    context.expose({
      getValue: () => fieldRef.value?.getValue() ?? normalizeControlString(props.value),
      setValue: (value: string) => fieldRef.value?.setValue(value),
      focus: () => fieldRef.value?.focus(),
      clear: () => fieldRef.value?.clear(),
      validate: () => true,
    } satisfies SearchFieldHandle);

    return () => (
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
        disabled={props.disabled}
        readonly={props.readonly}
        size={props.size}
        locale={props.locale}
        leadingIcon="search"
        clearable
        clearLabel={props.clearLabel}
        inputType="search"
        onEnter={props.onEnter}
        className={`field--search${props.className ? ` ${props.className}` : ''}`}
      />
    );
  },
);

export default SearchField;
</script>
