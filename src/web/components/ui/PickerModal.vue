<script lang="tsx">
import { computed, ref } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';

import { Modal } from './Modal.vue';
import { IconClose } from '../icons/index.ts';
import { SearchInput, TextInput } from './TextInput.vue';

export type PickerOption = {
  /** What gets stored in the filter. */
  value: string;
  label: string;
  /** Second line: the price of a gift, the points of a viewer… */
  meta?: VNodeChild;
  imageUrl?: string;
  /** Drawn when there is no image (or it fails to load). */
  fallback?: VNodeChild;
  /** Extra words the search should match — a gift id, a nickname. */
  keywords?: string;
};

export type PickerModalProps = {
  title: string;
  description?: string;
  options: PickerOption[];
  /** Values already chosen; they show as selected. */
  selected: string[];
  multiple?: boolean;
  searchPlaceholder: string;
  emptyLabel: string;
  /** Lets someone type a value the list does not have. */
  manualLabel?: string;
  manualPlaceholder?: string;
  /** Said when what is being typed already exists in the list. */
  alreadyLabel?: string;
  doneLabel: string;
  closeLabel: string;
  onPick: (values: string[]) => void;
  onClose: () => void;
};

/**
 * One searchable list of options with a picture per row — the gift picker and
 * the viewer picker are this component with a different source.
 */
export const PickerModal = defineVueComponent<PickerModalProps>(
  ['title', 'description', 'options', 'selected', 'multiple', 'searchPlaceholder', 'emptyLabel', 'manualLabel', 'manualPlaceholder', 'alreadyLabel', 'doneLabel', 'closeLabel', 'onPick', 'onClose'],
  (props) => {
  const query = ref('');
  const manual = ref('');
  /** With nothing matching, what was searched is what would be typed. */
  const manualDraft = computed(() => manual.value || (props.options.length > 0 ? query.value.trim() : ''));
  const chosen = ref<string[]>([...props.selected]);
  const broken = ref<Record<string, boolean>>({});

  const visible = computed(() => {
    const needle = query.value.trim().toLowerCase();
    if (!needle) return props.options;
    return props.options.filter((option) =>
      option.label.toLowerCase().includes(needle)
      || option.value.toLowerCase().includes(needle)
      || (option.keywords?.toLowerCase().includes(needle) ?? false));
  });

  const toggle = (value: string): void => {
    if (!props.multiple) {
      props.onPick([value]);
      return;
    }
    chosen.value = chosen.value.includes(value)
      ? chosen.value.filter((entry) => entry !== value)
      : [...chosen.value, value];
  };

  const manualValue = computed(() => manualDraft.value.trim());
  /** What the list already calls this, so typing "rosa" picks "Rosa" instead of adding it twice. */
  const existing = computed(() => props.options.find((option) => option.value.toLowerCase() === manualValue.value.toLowerCase()
    || option.label.toLowerCase() === manualValue.value.toLowerCase()));

  const addManual = (): void => {
    if (!manualValue.value) return;
    const value = existing.value?.value ?? manualValue.value;
    manual.value = '';
    query.value = '';
    if (!props.multiple) {
      props.onPick([value]);
      return;
    }
    chosen.value = chosen.value.includes(value) ? chosen.value : [...chosen.value, value];
  };

  return () => {
    const { title, description, selected, multiple = false, searchPlaceholder, emptyLabel, manualLabel, manualPlaceholder, alreadyLabel, doneLabel, closeLabel, onPick, onClose } = props;
    return (
    <Modal
      title={title}
      description={description}
      onClose={onClose}
      closeLabel={closeLabel}
      class="ui-picker"
      footer={multiple
        ? (
          <div class="ui-modal-card__actions">
            <button type="button" class="plg-btn plg-btn--sm" onClick={onClose}>{closeLabel}</button>
            <button type="button" class="plg-btn plg-btn--primary plg-btn--sm" onClick={() => onPick(chosen.value)}>
              {doneLabel}
            </button>
          </div>
        )
        : undefined}
    >
      <div class="ui-picker__tools">
        <SearchInput
          name="pickerSearch"
          value={query.value}
          onValueChange={(next) => { query.value = next; }}
          placeholder={searchPlaceholder}
        />
      </div>

      <div class="ui-picker__list">
        {visible.value.map((option) => {
          const active = multiple ? chosen.value.includes(option.value) : selected.includes(option.value);
          const showImage = option.imageUrl && !broken.value[option.value];
          return (
            <button
              type="button"
              key={option.value}
              class={`ui-picker__item${active ? ' is-active' : ''}`}
              onClick={() => toggle(option.value)}
            >
              <span class="ui-picker__thumb">
                {showImage
                  ? (
                    <img
                      src={option.imageUrl}
                      alt=""
                      loading="lazy"
                      onError={() => (broken.value = { ...broken.value, [option.value]: true })}
                    />
                  )
                  : option.fallback}
              </span>
              <span class="ui-picker__text">
                <span class="ui-picker__label">{option.label}</span>
                {option.meta && <span class="ui-picker__meta">{option.meta}</span>}
              </span>
              {active && <span class="ui-picker__check" aria-hidden="true">✓</span>}
            </button>
          );
        })}

        {visible.value.length === 0 && <p class="ui-picker__empty">{emptyLabel}</p>}
      </div>

      {/* Typing a name is the fallback, not the habit: it only shows up when the
          list has nothing to offer, or nothing matches what is being searched. */}
      {manualLabel && (props.options.length === 0 || (query.value.trim().length > 0 && visible.value.length === 0)) && (
        <div class="ui-picker__manual">
          <label class="plg-label" for="pickerManualValue">{manualLabel}</label>
          <div class="ui-picker__manual-row">
            <TextInput
              id="pickerManualValue"
              name="manualValue"
              value={manualDraft.value}
              onValueChange={(next) => { manual.value = next; }}
              placeholder={manualPlaceholder}
              onEnter={addManual}
            />
            <button type="button" class="plg-btn plg-btn--sm" onClick={addManual} disabled={!manualValue.value}>
              +
            </button>
          </div>
          {existing.value && <span class="ui-picker__hint">{alreadyLabel ?? ''}</span>}
        </div>
      )}

      {multiple && chosen.value.length > 0 && (
        <div class="ui-picker__chosen">
          {chosen.value.map((value) => (
            <button
              type="button"
              class="plg-pill plg-pill--accent ui-picker__chip"
              key={value}
              onClick={() => (chosen.value = chosen.value.filter((entry) => entry !== value))}
            >
              {props.options.find((option) => option.value === value)?.label ?? value}
              <span aria-hidden="true" class="ui-picker__chip-icon"><IconClose size={10} /></span>
            </button>
          ))}
        </div>
      )}
    </Modal>
    );
  };
  },
);

export default PickerModal;
</script>
