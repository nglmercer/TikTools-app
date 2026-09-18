<script lang="tsx">
import { nextTick, onUnmounted, ref, watch } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { Icon } from '../icons/index.ts';
import { fieldControlId, fieldMessageIds, describeField, type FieldSize } from './fields/field-logic.ts';
import { FieldShell } from './fields/FieldShell.vue';
import { InputGroup } from './fields/InputGroup.vue';
import { AutocompletePopover } from '../autocomplete/AutocompletePopover.vue';
import { suggestionOptionId } from '../autocomplete/types.ts';
import type { TooltipPosition } from './tooltip-logic.ts';

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
  ariaLabel?: string;
  className?: string;
  /** Shown when the value matches no option (a hand-written path). */
  placeholder?: string;
  disabled?: boolean;
  invalid?: boolean;
  size?: FieldSize;
  id?: string;
  label?: string;
  hint?: string;
  hintPosition?: TooltipPosition;
  description?: string;
  error?: string;
  required?: boolean;
};

let iconSelectFallback = 0;

/**
 * Icon-capable select on the canonical field system: FieldShell owns
 * label/required/hint/description/error/border/background/radius/focus/
 * disabled/size; InputGroup owns the row layout; the dropdown floats
 * through the shared AutocompletePopover (same positioning, z-index,
 * theme and field-anchored sizing as every autocomplete) and renders
 * with the shared autocomplete-list classes — no duplicated popup CSS.
 *
 * Native `<option>` cannot draw SVGs, which is why this custom listbox
 * exists alongside the native SelectField.
 */
export const IconSelect = defineVueComponent<IconSelectProps>(
  ['value', 'options', 'onChange', 'ariaLabel', 'className', 'placeholder', 'disabled', 'invalid', 'size', 'id', 'label', 'hint', 'hintPosition', 'description', 'error', 'required'],
  (props) => {
  const open = ref(false);
  const active = ref(0);
  const buttonRef = ref<HTMLButtonElement | null>(null);
  iconSelectFallback += 1;
  const fallbackId = `tt-iconselect-${iconSelectFallback}`;
  const listId = `${fallbackId}-listbox`;

  const selectedIndex = (): number => Math.max(0, props.options.findIndex((option) => option.value === props.value));

  const focusButton = (): void => {
    buttonRef.value?.focus();
  };

  const openAt = (): void => {
    if (props.disabled) return;
    active.value = selectedIndex();
    open.value = true;
    void nextTick(() => {
      document.addEventListener('pointerdown', onPointerDown, true);
    });
  };

  const close = (restoreFocus: boolean): void => {
    if (!open.value) return;
    open.value = false;
    document.removeEventListener('pointerdown', onPointerDown, true);
    if (restoreFocus) {
      void nextTick(() => focusButton());
    }
  };

  const commit = (index: number): void => {
    if (props.disabled) return;
    const option = props.options[index];
    if (option && !props.disabled) props.onChange(option.value);
    close(true);
  };

  const onPointerDown = (event: Event): void => {
    const target = event.target as Node | null;
    const button = buttonRef.value;
    const box = button?.closest?.('.field__box') as HTMLElement | null;
    // Clicks on the control toggle; outside clicks close; option picks
    // commit via click and must not be swallowed here.
    if (button && target && button.contains(target)) return;
    const popover = document.querySelector('.autocomplete-popover');
    if (popover && target && popover.contains(target)) return;
    if (box && target && box.contains(target)) return;
    close(false);
  };

  const onKeydown = (event: KeyboardEvent): void => {
    if (props.disabled) return;
    if (event.key === 'Escape') {
      if (open.value) {
        event.preventDefault();
        close(true);
      }
      return;
    }
    if (!open.value && (event.key === 'Enter' || event.key === ' ' || event.key === 'ArrowDown' || event.key === 'ArrowUp')) {
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
    } else if (event.key === 'Home') {
      event.preventDefault();
      active.value = 0;
    } else if (event.key === 'End') {
      event.preventDefault();
      active.value = Math.max(0, props.options.length - 1);
    } else if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      commit(active.value);
    } else if (event.key === 'Tab') {
      close(false);
    }
  };

  onUnmounted(() => {
    document.removeEventListener('pointerdown', onPointerDown, true);
  });

  watch(() => props.disabled, (disabled) => {
    if (disabled) close(false);
  });

  return () => {
    const { value, options, placeholder } = props;
    const id = fieldControlId(props, fallbackId);
    const invalid = props.invalid ?? Boolean(props.error);
    const { descriptionId, errorId } = fieldMessageIds(id);
    const label = props.label ?? props.ariaLabel;
    const selected = options.find((option) => option.value === value);
    const describedBy = describeField([props.description ? descriptionId : undefined, props.error ? errorId : undefined]);
    const anchor = (() => {
      const button = buttonRef.value;
      if (!button) return null;
      const box = button.closest?.('.field__box');
      return (box as HTMLElement | null) ?? button;
    })();
    return (
    <>
      <FieldShell
        id={id}
        label={props.label}
        hint={props.hint}
        hintPosition={props.hintPosition}
        description={props.description}
        error={props.error}
        required={props.required}
        disabled={props.disabled}
        invalid={invalid}
        size={props.size}
        className={props.className}
      >
        <InputGroup
          invalid={invalid}
          disabled={props.disabled}
          trailing={<Icon name="arrow-down" size={12} />}
        >
          <button
            ref={buttonRef}
            type="button"
            id={id}
            class="field-input field-input--select ui-icon-select__button"
            role="combobox"
            aria-label={props.ariaLabel ?? props.label}
            aria-haspopup="listbox"
            aria-expanded={open.value}
            aria-controls={open.value ? listId : undefined}
            aria-activedescendant={open.value && options.length > 0 ? suggestionOptionId(listId, active.value) : undefined}
            aria-invalid={invalid}
            aria-describedby={describedBy}
            aria-errormessage={props.error ? errorId : undefined}
            aria-disabled={props.disabled}
            disabled={props.disabled}
            onClick={() => (open.value ? close(false) : openAt())}
            onKeydown={onKeydown}
          >
            {selected?.icon && <span class="ui-icon-select__icon" aria-hidden="true">{selected.icon}</span>}
            <span class="ui-icon-select__value">{selected?.label ?? placeholder ?? ''}</span>
          </button>
        </InputGroup>
      </FieldShell>
      <AutocompletePopover
        anchor={anchor}
        open={open.value && !props.disabled}
        updateKey={`${value}:${options.length}`}
        anchorMode="field"
        onPopupPointerChange={() => {}}
      >
        <div id={listId} class="autocomplete-list" role="listbox" aria-label={props.ariaLabel ?? label ?? 'Options'}>
          {options.map((option, index) => (
            <button
              type="button"
              key={option.value}
              id={suggestionOptionId(listId, index)}
              role="option"
              aria-selected={option.value === value}
              class={`autocomplete-item${option.value === value ? ' is-selected' : ''}${index === active.value ? ' is-selected' : ''}`}
              title={option.hint}
              onMouseenter={() => (active.value = index)}
              onClick={() => commit(index)}
            >
              {option.icon && <span class="autocomplete-item__icon" aria-hidden="true">{option.icon}</span>}
              <span class="autocomplete-item__body">
                <span class="autocomplete-item__title">
                  <span class="autocomplete-item__label">{option.label}</span>
                </span>
                {option.meta && <span class="autocomplete-item__desc">{option.meta}</span>}
              </span>
            </button>
          ))}
        </div>
      </AutocompletePopover>
    </>
    );
  };
  },
);

export default IconSelect;
</script>
