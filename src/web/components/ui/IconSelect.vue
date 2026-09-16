<script lang="tsx">
import { onMounted, onUnmounted, ref } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

export type IconSelectOption = {
  value: string;
  label: string;
  /** Drawn inside the closed control and beside the option. */
  icon?: VNodeChild;
  /** Second line in the list: the code equivalent, a sample value… */
  meta?: string;
  hint?: string;
};

type IconSelectProps = {
  value: string;
  options: IconSelectOption[];
  onChange: (value: string) => void;
  ariaLabel: string;
  className?: string;
  /** Shown when the value matches no option (a hand-written path). */
  placeholder?: string;
};

/**
 * A select that can draw an icon. The native one cannot, which is why the
 * icon used to sit outside as a second element; here the closed control shows
 * the selected option exactly as the list does, so there is only one of it.
 */
export const IconSelect = defineVueComponent<IconSelectProps>(
  ['value', 'options', 'onChange', 'ariaLabel', 'className', 'placeholder'],
  (props) => {
  const open = ref(false);
  const active = ref(0);
  const rootRef = ref<HTMLDivElement | null>(null);
  const onPointerDown = (event: MouseEvent): void => {
    if (open.value && !rootRef.value?.contains(event.target as Node)) open.value = false;
  };

  onMounted(() => document.addEventListener('mousedown', onPointerDown));
  onUnmounted(() => document.removeEventListener('mousedown', onPointerDown));

  const openAt = (): void => {
    active.value = Math.max(0, props.options.findIndex((option) => option.value === props.value));
    open.value = true;
  };

  const commit = (index: number): void => {
    const option = props.options[index];
    if (option) props.onChange(option.value);
    open.value = false;
  };

  const onKeydown = (event: KeyboardEvent): void => {
    if (event.key === 'Escape') {
      open.value = false;
      return;
    }
    if (!open.value && (event.key === 'Enter' || event.key === ' ' || event.key === 'ArrowDown')) {
      event.preventDefault();
      openAt();
      return;
    }
    if (!open.value) return;
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      active.value = Math.min(props.options.length - 1, active.value + 1);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      active.value = Math.max(0, active.value - 1);
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      commit(active.value);
    }
  };

  return () => {
    const { value, options, ariaLabel, className = '', placeholder } = props;
    const selected = options.find((option) => option.value === value);
    return (
    <div class={`ui-icon-select ${className}`.trim()} ref={rootRef}>
      <button
        type="button"
        class={`ui-icon-select__control${open.value ? ' is-open' : ''}`}
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open.value}
        onClick={() => (open.value ? (open.value = false) : openAt())}
        onKeydown={onKeydown}
      >
        {selected?.icon && <span class="ui-icon-select__icon">{selected.icon}</span>}
        <span class="ui-icon-select__value">{selected?.label ?? placeholder ?? ''}</span>
        <svg
          class="ui-icon-select__caret"
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
          aria-hidden="true"
        >
          <path d="m6 9 6 6 6-6" />
        </svg>
      </button>

      {open.value && (
        <div class="ui-icon-select__menu" role="listbox" aria-label={ariaLabel}>
          {options.map((option, index) => (
            <button
              type="button"
              key={option.value}
              role="option"
              aria-selected={option.value === value}
              class={`ui-icon-select__option${option.value === value ? ' is-selected' : ''}${index === active.value ? ' is-active' : ''}`}
              title={option.hint}
              onMouseenter={() => (active.value = index)}
              onClick={() => commit(index)}
            >
              {option.icon && <span class="ui-icon-select__icon">{option.icon}</span>}
              <span class="ui-icon-select__text">
                <span class="ui-icon-select__label">{option.label}</span>
                {option.meta && <span class="ui-icon-select__meta">{option.meta}</span>}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
    );
  };
  },
);

export default IconSelect;
</script>
