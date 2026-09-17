<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import type { Locale } from '../../i18n.ts';
import { NumberField, type NumberFieldHandle } from './fields/index.ts';
import type { FieldSize } from './fields/field-logic.ts';

export type NumberInputHandle = {
  getValue: () => number | null;
  setValue: (v: number | null) => void;
  focus: () => void;
};

type NumberInputProps = {
  value: number | null;
  onValueChange: (v: number | null) => void;
  min?: number;
  max?: number;
  step?: number;
  disabled?: boolean;
  readonly?: boolean;
  required?: boolean;
  error?: string;
  id?: string;
  name?: string;
  suffix?: string;
  placeholder?: string;
  /** Label-above-input. (Previously a floating label; the prop is unchanged.) */
  label?: string;
  /** Tooltip-only explanation (ⓘ). */
  hint?: string;
  size?: FieldSize;
  locale?: Locale;
};

/**
 * Compatibility shim over NumberField: identical props/handles, shared shell
 * with draft-preserving typing and accessible steppers.
 */
export const NumberInput = defineVueComponent<NumberInputProps>(
  ['value', 'onValueChange', 'min', 'max', 'step', 'disabled', 'readonly', 'required', 'error', 'id', 'name', 'suffix', 'placeholder', 'label', 'hint', 'size', 'locale'],
  (props, context) => {
    const fieldRef = ref<NumberFieldHandle | null>(null);
    context.expose({
      getValue: () => fieldRef.value?.getValue() ?? props.value,
      setValue: (value: number | null) => fieldRef.value?.setValue(value),
      focus: () => fieldRef.value?.focus(),
    } satisfies NumberInputHandle);

    return () => (
      <NumberField
        ref={fieldRef}
        value={props.value}
        onValueChange={props.onValueChange}
        min={props.min}
        max={props.max}
        step={props.step}
        disabled={props.disabled}
        readonly={props.readonly}
        required={props.required}
        error={props.error}
        id={props.id}
        name={props.name}
        suffix={props.suffix}
        placeholder={props.placeholder}
        label={props.label}
        hint={props.hint}
        size={props.size}
        locale={props.locale}
      />
    );
  },
);

export default NumberInput;
</script>
