<script lang="tsx">
import { computed, onBeforeUnmount, ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { FormField } from './FormField.vue';
import { IconClose } from '../icons/index.ts';
import { filterComboOptions } from './controls.ts';
import { dispatchControlEvent, normalizeControlString, syncNativeControlValue } from './control-events.ts';
import { fieldIds, type SelectOption } from './controls.ts';

type ComboboxProps = {
  value: string;
  onValueChange: (v: string) => void;
  options: SelectOption[];
  label?: string;
  hint?: string;
  placeholder?: string;
  disabled?: boolean;
  required?: boolean;
  error?: string;
  id?: string;
  name?: string;
  clearable?: boolean;
};

let comboFallback = 0;


export const Combobox = defineVueComponent<ComboboxProps>(
  ['value', 'onValueChange', 'options', 'label', 'hint', 'placeholder', 'disabled', 'required', 'error', 'id', 'name', 'clearable'],
  (props, context) => {
  const innerRef = ref<HTMLInputElement | null>(null);
  const open = ref(false);
  const active = ref(-1);
  const rootRef = ref<HTMLDivElement | null>(null);
  const listId = ref('');
  const query = ref(normalizeControlString(props.value));
  const commitProgrammaticValue = (value: string): void => {
    query.value = value;
    const control = innerRef.value;
    if (control) {
      syncNativeControlValue(control, value);
      dispatchControlEvent(control);
    }
    props.onValueChange(value);
  };
  context.expose({
    getValue: () => normalizeControlString(props.value),
    setValue: commitProgrammaticValue,
    focus: () => innerRef.value?.focus(),
    clear: () => commitProgrammaticValue(''),
  });
  const close = (refocus = false): void => {
    open.value = false;
    active.value = -1;
    if (refocus) innerRef.value?.focus();
  };
  const onOutside = (event: Event): void => {
    if (rootRef.value && !rootRef.value.contains(event.target as Node)) close();
  };
  if (typeof document !== 'undefined') {
    document.addEventListener('pointerdown', onOutside, true);
  }
  onBeforeUnmount(() => {
    if (typeof document !== 'undefined') document.removeEventListener('pointerdown', onOutside, true);
  });
  return () => {
    comboFallback += 1;
    const { id, describedBy } = fieldIds(props, `tt-combo-${comboFallback}`);
    if (!listId.value) listId.value = `${id}-listbox`;
    const filtered = computed(() => filterComboOptions(props.options, query.value));
    const labelOf = (v: string): string => props.options.find((o) => o.value === v)?.label ?? v;
    const hintId = props.hint ? `${id}-hint` : undefined;
    const errorId = props.error ? `${id}-error` : undefined;
    const choose = (option: SelectOption): void => {
      if (option.disabled) return;
      commitProgrammaticValue(option.value);
      close(true);
    };
    const control = (
      <div ref={rootRef} class={`ui-combobox ${open.value ? 'is-open' : ''} ${props.error ? 'has-error' : ''} ${props.disabled ? 'is-disabled' : ''}`}>
        {props.name ? <input type="hidden" name={props.name} value={normalizeControlString(props.value)} /> : null}
        <div class="ui-combobox__control">
          <input
            ref={innerRef}
            id={id}
            role="combobox"
            type="text"
            value={query.value}
            placeholder={props.placeholder}
            disabled={props.disabled}
            required={props.required}
            autocomplete="off"
            spellcheck={false}
            aria-expanded={open.value}
            aria-controls={listId.value}
            aria-activedescendant={active.value >= 0 ? `${listId.value}-${active.value}` : undefined}
            aria-invalid={Boolean(props.error)}
            aria-describedby={describedBy([hintId, errorId])}
            onInput={(e) => {
              query.value = (e.currentTarget as HTMLInputElement).value;
              open.value = true;
              active.value = -1;
            }}
            onFocus={() => {
              query.value = labelOf(normalizeControlString(props.value)) === normalizeControlString(props.value)
                ? normalizeControlString(props.value)
                : '';
              if (query.value === normalizeControlString(props.value)) query.value = '';
              open.value = true;
            }}
            onBlur={() => {
              const current = normalizeControlString(props.value);
              if (query.value === '' && current !== '') query.value = labelOf(current);
            }}
            onKeydown={(e) => {
              const list = filtered.value;
              if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
                e.preventDefault();
                if (!open.value) {
                  open.value = true;
                  return;
                }
                const dir = e.key === 'ArrowDown' ? 1 : -1;
                let next = active.value + dir;
                for (let i = 0; i < list.length; i += 1) {
                  const wrapped = (next + list.length) % list.length;
                  if (!list[wrapped]?.disabled) {
                    active.value = wrapped;
                    break;
                  }
                  next += dir;
                }
              } else if (e.key === 'Enter') {
                const option = active.value >= 0 ? list[active.value] : list.find((o) => o.label.toLowerCase() === query.value.toLowerCase() || o.value.toLowerCase() === query.value.toLowerCase());
                if (open.value && option && !option.disabled) {
                  e.preventDefault();
                  choose(option);
                } else if (!open.value) {
                  commitProgrammaticValue(query.value);
                }
              } else if (e.key === 'Escape') {
                e.preventDefault();
                close(true);
              }
            }}
          />
          {props.clearable !== false && query.value ? (
            <button
              type="button"
              class="ui-combobox__clear"
              aria-label="Clear selection"
              disabled={props.disabled}
              onClick={() => commitProgrammaticValue('')}
            >
              <IconClose size={10} />
            </button>
          ) : null}
          <button
            type="button"
            class="ui-combobox__arrow"
            aria-label={open.value ? 'Close options' : 'Open options'}
            aria-expanded={open.value}
            disabled={props.disabled}
            onClick={() => {
              open.value = !open.value;
              if (open.value) innerRef.value?.focus();
            }}
          >
            ▾
          </button>
        </div>
        {open.value && !props.disabled ? (
          <ul id={listId.value} role="listbox" class="ui-combobox__list" aria-label={props.label ?? 'Options'}>
            {filtered.value.length === 0 ? (
              <li class="ui-combobox__empty" role="option" aria-selected={false} aria-disabled>No results for “{query.value}”</li>
            ) : filtered.value.map((option, index) => (
              <li
                key={option.value}
                id={`${listId.value}-${index}`}
                role="option"
                aria-selected={props.value === option.value}
                aria-disabled={option.disabled}
                class={`ui-combobox__option ${active.value === index ? 'is-active' : ''} ${props.value === option.value ? 'is-selected' : ''} ${option.disabled ? 'is-disabled' : ''}`}
                onMouseenter={() => { if (!option.disabled) active.value = index; }}
                onMousedown={(e) => {
                  e.preventDefault();
                  choose(option);
                }}
              >
                {option.label}
              </li>
            ))}
          </ul>
        ) : null}
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

export default Combobox;
</script>
