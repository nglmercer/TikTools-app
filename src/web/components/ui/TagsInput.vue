<script lang="tsx">
import { ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { FormField } from './FormField.vue';
import { IconClose } from '../icons/index.ts';
import { addTags, fieldIds, splitTagCandidates } from './controls.ts';

type TagsInputProps = {
  value: string[];
  onValueChange: (v: string[]) => void;
  label?: string;
  hint?: string;
  placeholder?: string;
  disabled?: boolean;
  readonly?: boolean;
  required?: boolean;
  error?: string;
  id?: string;
  name?: string;
  maxTags?: number;
  ariaLabel?: string;
  allowComma?: boolean;
  validate?: (tag: string) => boolean;
};

let tagsFallback = 0;

export const TagsInput = defineVueComponent<TagsInputProps>(
  ['value', 'onValueChange', 'label', 'hint', 'placeholder', 'disabled', 'readonly', 'required', 'error', 'id', 'name', 'maxTags', 'ariaLabel', 'allowComma', 'validate'],
  (props, context) => {
  const draft = ref('');
  const innerRef = ref<HTMLInputElement | null>(null);
  context.expose({
    getValue: () => [...props.value],
    setValue: (v: string[]) => props.onValueChange([...v]),
    focus: () => innerRef.value?.focus(),
    clear: () => props.onValueChange([]),
  });
  const commit = (raw: string): void => {
    const candidates = props.allowComma === false ? [raw.trim()].filter((t) => t.length > 0) : splitTagCandidates(raw);
    if (candidates.length === 0) return;
    const next = addTags(props.value, candidates, { maxTags: props.maxTags, validate: props.validate });
    if (next.added.length > 0) props.onValueChange(next.tags);
    draft.value = '';
  };
  const removeAt = (index: number): void => {
    if (props.disabled) return;
    props.onValueChange(props.value.filter((_, i) => i !== index));
  };
  return () => {
    tagsFallback += 1;
    const { id, describedBy } = fieldIds(props, `tt-tags-${tagsFallback}`);
    const hintId = props.hint ? `${id}-hint` : undefined;
    const errorId = props.error ? `${id}-error` : undefined;
    const maxed = props.maxTags !== undefined && props.value.length >= props.maxTags;
    const control = (
      <div class={`ui-tags ${props.error ? 'has-error' : ''} ${props.disabled ? 'is-disabled' : ''}`}>
        <div class="ui-tags__list" role="list" aria-label={props.label ?? props.ariaLabel ?? 'Tags'}>
          {props.value.map((tag, index) => (
            <span key={`${tag}-${index}`} class="ui-tags__tag" role="listitem">
              <span class="ui-tags__text">{tag}</span>
              <button
                type="button"
                class="ui-tags__remove"
                aria-label={`Remove ${tag}`}
                disabled={props.disabled}
                onClick={() => removeAt(index)}
              >
                <IconClose size={10} />
              </button>
            </span>
          ))}
          {props.value.length === 0 ? <span class="ui-tags__empty">No tags yet</span> : null}
        </div>
        <input
          ref={innerRef}
          id={id}
          name={props.name}
          value={draft.value}
          placeholder={props.placeholder ?? (maxed ? 'Tag limit reached' : 'Add a tag and press Enter')}
          disabled={props.disabled || maxed}
          readonly={props.readonly}
          required={props.required && props.value.length === 0}
          aria-invalid={Boolean(props.error)}
          aria-describedby={describedBy([hintId, errorId])}
          onInput={(e) => {
            const next = (e.currentTarget as HTMLInputElement).value;
            if (props.allowComma !== false && next.includes(',')) {
              commit(next);
              return;
            }
            draft.value = next;
          }}
          onKeydown={(e) => {
            const target = e.currentTarget as HTMLInputElement;
            if (e.key === 'Enter') {
              e.preventDefault();
              commit(target.value);
            } else if (e.key === 'Backspace' && target.value === '' && props.value.length > 0) {
              removeAt(props.value.length - 1);
            }
          }}
          onBlur={() => {
            if (draft.value.trim()) commit(draft.value);
          }}
        />
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

export default TagsInput;
</script>
