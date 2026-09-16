<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { FormField } from './FormField.vue';
import { IconClose } from '../icons/index.ts';
import { clampNumber } from './controls.ts';

type RatingProps = {
  value: number;
  onValueChange: (v: number) => void;
  max?: number;
  label?: string;
  hint?: string;
  error?: string;
  disabled?: boolean;
  required?: boolean;
  id?: string;
  name?: string;
  allowClear?: boolean;
};

export const Rating = defineVueComponent<RatingProps>(
  ['value', 'onValueChange', 'max', 'label', 'hint', 'error', 'disabled', 'required', 'id', 'name', 'allowClear'],
  (props, context) => {
  const hovered = ref(0);
  context.expose({
    getValue: () => props.value,
    setValue: (v: number) => props.onValueChange(clampNumber(Math.round(v), 0, props.max ?? 5)),
    clear: () => props.onValueChange(0),
    focus: () => undefined,
  });
  return () => {
    const max = props.max ?? 5;
    const preview = hovered.value || props.value;
    const group = (
      <div
        class={`ui-rating ${props.disabled ? 'is-disabled' : ''} ${props.error ? 'has-error' : ''}`}
        role="radiogroup"
        aria-label={props.label ?? 'Rating'}
        aria-invalid={Boolean(props.error)}
        onMouseleave={() => { hovered.value = 0; }}
      >
        {props.name ? <input type="hidden" name={props.name} value={String(props.value)} /> : null}
        {Array.from({ length: max }, (_, i) => i + 1).map((star) => {
          const active = star <= preview;
          return (
            <button
              key={star}
              type="button"
              role="radio"
              aria-checked={props.value === star}
              aria-label={`Rate ${star} of ${max}`}
              class={`ui-rating__star ${active ? 'is-active' : ''} ${props.value === star ? 'is-selected' : ''}`}
              disabled={props.disabled}
              onClick={() => {
                if (props.allowClear !== false && props.value === star) props.onValueChange(0);
                else props.onValueChange(star);
              }}
              onMouseenter={() => { hovered.value = star; }}
              onFocus={() => { hovered.value = star; }}
              onBlur={() => { hovered.value = 0; }}
              onKeydown={(e) => {
                if (e.key === 'ArrowRight' || e.key === 'ArrowUp') {
                  e.preventDefault();
                  props.onValueChange(clampNumber(star + 1 <= max ? star + 1 : star, 0, max));
                } else if (e.key === 'ArrowLeft' || e.key === 'ArrowDown') {
                  e.preventDefault();
                  props.onValueChange(clampNumber(star - 1, 0, max));
                } else if ((e.key === 'Delete' || e.key === 'Backspace') && props.allowClear !== false) {
                  e.preventDefault();
                  props.onValueChange(0);
                }
              }}
            >
              <span aria-hidden>{active ? '★' : '☆'}</span>
            </button>
          );
        })}
        {props.allowClear !== false && props.value > 0 ? (
          <button type="button" class="ui-rating__clear" disabled={props.disabled} aria-label="Clear rating" onClick={() => props.onValueChange(0)}><IconClose size={10} /></button>
        ) : null}
      </div>
    );
    if (!props.label && !props.hint && !props.error) return group;
    return (
      <FormField label={props.label} hint={props.hint} error={props.error} required={props.required}>
        {group}
      </FormField>
    );
  };
  },
);

export default Rating;
</script>
