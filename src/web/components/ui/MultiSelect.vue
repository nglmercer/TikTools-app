<script lang="tsx">
import { computed, onBeforeUnmount, ref } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { FormField } from './FormField.vue';
import { IconClose } from '../icons/index.ts';
import { filterMultiOptions } from './controls.ts';
import { fieldIds, type SelectOption } from './controls.ts';

type MultiSelectProps = {
  value: string[];
  onValueChange: (v: string[]) => void;
  options: SelectOption[];
  label?: string;
  hint?: string;
  placeholder?: string;
  disabled?: boolean;
  required?: boolean;
  error?: string;
  id?: string;
  name?: string;
  searchable?: boolean;
  clearable?: boolean;
};

let multiFallback = 0;


export const MultiSelect = defineVueComponent<MultiSelectProps>(
  ['value', 'onValueChange', 'options', 'label', 'hint', 'placeholder', 'disabled', 'required', 'error', 'id', 'name', 'searchable', 'clearable'],
  (props, context) => {
  const open = ref(false);
  const active = ref(-1);
  const query = ref('');
  const rootRef = ref<HTMLDivElement | null>(null);
  const searchRef = ref<HTMLInputElement | null>(null);
  const listId = ref('');
  context.expose({
    getValue: () => [...props.value],
    setValue: (v: string[]) => props.onValueChange([...v]),
    clear: () => props.onValueChange([]),
    focus: () => searchRef.value?.focus(),
  });
  const close = (): void => {
    open.value = false;
    active.value = -1;
  };
  const onOutside = (event: Event): void => {
    if (rootRef.value && !rootRef.value.contains(event.target as Node)) close();
  };
  if (typeof document !== 'undefined') document.addEventListener('pointerdown', onOutside, true);
  onBeforeUnmount(() => {
    if (typeof document !== 'undefined') document.removeEventListener('pointerdown', onOutside, true);
  });
  const toggle = (option: SelectOption): void => {
    if (props.disabled || option.disabled) return;
    const exists = props.value.includes(option.value);
    props.onValueChange(exists ? props.value.filter((v) => v !== option.value) : [...props.value, option.value]);
  };
  return () => {
    multiFallback += 1;
    const { id, describedBy } = fieldIds(props, `tt-multi-${multiFallback}`);
    if (!listId.value) listId.value = `${id}-listbox`;
    const filtered = computed(() => filterMultiOptions(props.options, query.value).filter((o) => !props.value.includes(o.value) || true));
    const hintId = props.hint ? `${id}-hint` : undefined;
    const errorId = props.error ? `${id}-error` : undefined;
    const labelOf = (v: string): string => props.options.find((o) => o.value === v)?.label ?? v;
    const control = (
      <div ref={rootRef} class={`ui-multi-select ${open.value ? 'is-open' : ''} ${props.error ? 'has-error' : ''} ${props.disabled ? 'is-disabled' : ''}`}>
        {props.name ? props.value.map((v) => <input key={v} type="hidden" name={props.name} value={v} />) : null}
        <div
          class="ui-multi-select__control"
          role="combobox"
          aria-expanded={open.value}
          aria-controls={listId.value}
          aria-invalid={Boolean(props.error)}
          aria-describedby={describedBy([hintId, errorId])}
          aria-label={props.label ?? 'Multi select'}
          tabindex={props.disabled ? -1 : 0}
          onClick={() => {
            if (props.disabled) return;
            open.value = !open.value;
            if (open.value) searchRef.value?.focus();
          }}
          onKeydown={(e) => {
            const list = filtered.value.filter((o) => !o.disabled);
            if (e.key === 'Escape') {
              e.preventDefault();
              close();
            } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
              e.preventDefault();
              open.value = true;
              const dir = e.key === 'ArrowDown' ? 1 : -1;
              active.value = list.length === 0 ? -1 : (active.value + dir + list.length) % list.length;
            } else if (e.key === 'Enter' && open.value && active.value >= 0) {
              e.preventDefault();
              const option = filtered.value[active.value];
              if (option) toggle(option);
            } else if (e.key === 'Backspace' && props.value.length > 0 && query.value === '') {
              props.onValueChange(props.value.slice(0, -1));
            }
          }}
        >
          <div class="ui-multi-select__chips">
            {props.value.length === 0 ? (
              <span class="ui-multi-select__placeholder">{props.placeholder ?? 'Select options'}</span>
            ) : props.value.map((v) => {
              const option = props.options.find((o) => o.value === v);
              const optionDisabled = option?.disabled;
              return (
                <span key={v} class="ui-multi-select__chip">
                  <span>{labelOf(v)}</span>
                  <button
                    type="button"
                    aria-label={`Remove ${labelOf(v)}`}
                    disabled={props.disabled || optionDisabled}
                    onClick={(e) => {
                      e.stopPropagation();
                      props.onValueChange(props.value.filter((entry) => entry !== v));
                    }}
                  >
                    <IconClose size={10} />
                  </button>
                </span>
              );
            })}
          </div>
          <span class="ui-multi-select__actions">
            {props.clearable !== false && props.value.length > 0 ? (
              <button
                type="button"
                aria-label="Clear all selected"
                disabled={props.disabled}
                onClick={(e) => {
                  e.stopPropagation();
                  props.onValueChange([]);
                }}
              >
                <IconClose size={10} />
              </button>
            ) : null}
            <span aria-hidden class="ui-multi-select__arrow">▾</span>
          </span>
        </div>
        {open.value && !props.disabled ? (
          <div class="ui-multi-select__dropdown">
            {props.searchable !== false ? (
              <input
                ref={searchRef}
                class="ui-multi-select__search"
                value={query.value}
                placeholder="Search options"
                role="searchbox"
                aria-label="Search options"
                onInput={(e) => {
                  query.value = (e.currentTarget as HTMLInputElement).value;
                  active.value = -1;
                }}
                onKeydown={(e) => e.stopPropagation()}
                onClick={(e) => e.stopPropagation()}
              />
            ) : null}
            <ul id={listId.value} role="listbox" aria-multiselectable={true} class="ui-multi-select__list" aria-label={props.label ?? 'Options'}>
              {filtered.value.length === 0 ? (
                <li class="ui-multi-select__empty" role="option" aria-selected={false}>No options found</li>
              ) : filtered.value.map((option, index) => {
                const selected = props.value.includes(option.value);
                return (
                  <li
                    key={option.value}
                    id={`${listId.value}-${index}`}
                    role="option"
                    aria-selected={selected}
                    aria-disabled={option.disabled}
                    class={`ui-multi-select__option ${selected ? 'is-selected' : ''} ${active.value === index ? 'is-active' : ''} ${option.disabled ? 'is-disabled' : ''}`}
                    onMouseenter={() => { if (!option.disabled) active.value = index; }}
                    onMousedown={(e) => {
                      e.preventDefault();
                      toggle(option);
                    }}
                  >
                    <span class="ui-multi-select__check" aria-hidden>{selected ? '✓' : ''}</span>
                    <span>{option.label}</span>
                  </li>
                );
              })}
            </ul>
          </div>
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

export default MultiSelect;
</script>
