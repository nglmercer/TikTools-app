<script lang="tsx">
import { ref, watch } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../../vue/component.ts';
import { Icon, type IconName } from '../../icons/index.ts';
import { t, type Locale } from '../../../i18n.ts';
import type { TooltipPosition } from '../tooltip-logic.ts';
import { dispatchControlEvent, normalizeControlString, syncNativeControlValue } from '../control-events.ts';
import { FieldShell } from './FieldShell.vue';
import { InputGroup } from './InputGroup.vue';
import { describeField, fieldControlId, fieldMessageIds, type FieldSize } from './field-logic.ts';
import { comboboxInputAttrs, type PresetItem } from '../../autocomplete/autocomplete-controller.ts';
import type { SuggestionItem } from '../../autocomplete/types.ts';
import { useAutocompleteInput } from '../../autocomplete/use-autocomplete.ts';
import { AutocompleteList } from '../../autocomplete/AutocompleteList.vue';
import { AutocompletePopover } from '../../autocomplete/AutocompletePopover.vue';

export type TextFieldHandle = {
  getValue: () => string;
  setValue: (v: string) => void;
  focus: () => void;
  clear: () => void;
  validate: () => boolean;
};

export type TextFieldProps = {
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
  required?: boolean;
  disabled?: boolean;
  readonly?: boolean;
  size?: FieldSize;
  locale?: Locale;
  ariaLabel?: string;
  /** Whitelisted decorative icon before the text. */
  leadingIcon?: IconName;
  /** Whitelisted decorative icon after the text. */
  trailingIcon?: IconName;
  /** Raw leading/trailing content (compat slots; prefer the IconName props). */
  leading?: VNodeChild;
  trailing?: VNodeChild;
  /** Semantic text pinned before the input (`@`, `$`). Not an icon. */
  prefix?: string;
  /** Semantic text pinned after the input (`ms`, `%`). Not an icon. */
  suffix?: string;
  clearable?: boolean;
  clearLabel?: string;
  inputType?: 'text' | 'password' | 'search' | 'url' | 'email' | 'tel';
  autoComplete?: string;
  spellCheck?: boolean;
  maxLength?: number;
  /**
   * URL presets (Quick destinations). Preset mode only — template variables
   * are NEVER suggested here (see TemplateField for those).
   */
  presets?: PresetItem[];
  /**
   * ComboBox options (recent creators, dynamic lists). When present this
   * field runs the options-mode controller: focus-empty opens the full
   * list, typing filters by label/value, click/Enter commits the value.
   * Takes precedence over `presets`; same field-anchored popover.
   */
  options?: SuggestionItem[];
  /** Open the options list on focus-empty (default true). */
  openOnFocus?: boolean;
  /** Fires with the committed option value (in addition to onValueChange). */
  onOptionPick?: (value: string) => void;
  onEnter?: () => void;
  onFocus?: () => void;
  onBlur?: () => void;
  className?: string;
};

let textFieldFallback = 0;

/**
 * Canonical single-line text field: FieldShell + InputGroup + native input.
 * Without `presets` it is pure text (zero suggestion behavior); with
 * `presets` it offers Quick-destination URL presets via the preset-mode
 * controller — which by construction never suggests template variables.
 */
export const TextField = defineVueComponent<TextFieldProps>(
  ['value', 'onValueChange', 'id', 'name', 'label', 'hint', 'hintPosition', 'description', 'error', 'placeholder', 'required', 'disabled', 'readonly', 'size', 'locale', 'ariaLabel', 'leadingIcon', 'trailingIcon', 'leading', 'trailing', 'prefix', 'suffix', 'clearable', 'clearLabel', 'inputType', 'autoComplete', 'spellCheck', 'maxLength', 'presets', 'options', 'openOnFocus', 'onOptionPick', 'onEnter', 'onFocus', 'onBlur', 'className'],
  (props, context) => {
    const innerRef = ref<HTMLInputElement | null>(null);
    const focused = ref(false);
    const cursor = ref(normalizeControlString(props.value).length);
    textFieldFallback += 1;
    const fallbackId = `tt-field-${textFieldFallback}`;
    const hasOptions = (): boolean => (props.options?.length ?? 0) > 0;
    const hasPresets = (): boolean => !hasOptions() && (props.presets?.length ?? 0) > 0;
    const autocomplete = useAutocompleteInput(() => ({
      mode: hasOptions() ? 'options' : 'preset',
      presets: props.presets,
      options: props.options,
      openOptionsOnFocus: props.openOnFocus,
      locale: props.locale ?? 'en',
    }));
    const readCaret = (): number => {
      const control = innerRef.value;
      return control?.selectionStart ?? normalizeControlString(props.value).length;
    };
    const commitProgrammaticValue = (value: string): void => {
      const control = innerRef.value;
      if (control) {
        syncNativeControlValue(control, value);
        dispatchControlEvent(control);
      }
      props.onValueChange(value);
      cursor.value = value.length;
      autocomplete.update(value, value.length, focused.value);
    };
    const applyPresetResult = (value: string, caret: number): void => {
      const control = innerRef.value;
      if (control) {
        syncNativeControlValue(control, value);
        dispatchControlEvent(control);
      }
      props.onValueChange(value);
      cursor.value = caret;
      requestAnimationFrame(() => {
        innerRef.value?.focus();
        innerRef.value?.setSelectionRange(caret, caret);
      });
    };
    context.expose({
      getValue: () => innerRef.value?.value ?? normalizeControlString(props.value),
      setValue: commitProgrammaticValue,
      focus: () => innerRef.value?.focus(),
      clear: () => commitProgrammaticValue(''),
      validate: () => !(props.required && !normalizeControlString(props.value).trim()),
    } satisfies TextFieldHandle);

    watch(() => props.value, (value) => {
      if (innerRef.value) syncNativeControlValue(innerRef.value, value);
      cursor.value = Math.min(cursor.value, normalizeControlString(value).length);
    });

    return () => {
      const id = fieldControlId(props, fallbackId);
      const invalid = Boolean(props.error);
      const { descriptionId, errorId } = fieldMessageIds(id);
      const locale = props.locale ?? 'en';
      const value = normalizeControlString(props.value);
      const suggestMode = hasOptions() || hasPresets();
      const snapshot = autocomplete.snapshot.value;
      const showClear = Boolean(props.clearable && value && !props.disabled && !props.readonly);
      const leading = props.leading ?? (props.leadingIcon ? <Icon name={props.leadingIcon} size={14} /> : undefined);
      const trailing = props.trailing ?? (props.trailingIcon ? <Icon name={props.trailingIcon} size={14} /> : undefined);
      const describedBy = describeField([props.description ? descriptionId : undefined, props.error ? errorId : undefined]);
      const comboAttrs = suggestMode
        ? comboboxInputAttrs({
          listId: autocomplete.listId,
          open: snapshot.open,
          activeIndex: snapshot.activeIndex,
          rowCount: snapshot.rowCount,
          describedBy,
          invalid,
          errorId: props.error ? errorId : undefined,
        })
        : {
          'aria-invalid': invalid,
          'aria-describedby': describedBy,
          'aria-errormessage': props.error ? errorId : undefined,
        } as const;
      const pushState = (next: string, caret: number, isFocused: boolean): void => {
        cursor.value = caret;
        if (suggestMode) autocomplete.update(next, caret, isFocused);
      };
      const applySuggestResult = (applied: { value: string; caret: number; row: { item: { value: string } } }): void => {
        applyPresetResult(applied.value, applied.caret);
        if (hasOptions()) props.onOptionPick?.(applied.row.item.value);
      };
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
            size={props.size}
            className={props.className}
          >
            <InputGroup
              leading={leading}
              trailing={trailing}
              prefix={props.prefix}
              suffix={props.suffix}
              focused={focused.value}
              invalid={invalid}
              disabled={props.disabled}
            >
              <input
                ref={innerRef}
                id={id}
                name={props.name}
                aria-label={props.ariaLabel}
                type={props.inputType ?? 'text'}
                value={value}
                placeholder={props.placeholder}
                disabled={props.disabled}
                readonly={props.readonly}
                autocomplete={props.autoComplete ?? 'off'}
                spellcheck={props.spellCheck ?? false}
                required={props.required}
                maxlength={props.maxLength}
                class="field-input"
                {...comboAttrs}
                onInput={(e) => {
                  const target = e.currentTarget as HTMLInputElement;
                  props.onValueChange(target.value);
                  pushState(target.value, target.selectionStart ?? target.value.length, true);
                }}
                onKeydown={(e) => {
                  if (suggestMode) {
                    const result = autocomplete.keydown(e);
                    if (result === 'commit') {
                      const caret = readCaret();
                      const applied = autocomplete.commit(value, caret);
                      if (applied) applySuggestResult(applied);
                      return;
                    }
                    if (result === 'dismissed') return;
                  }
                  if (e.key === 'Enter' && props.onEnter) props.onEnter();
                }}
                onKeyup={() => pushState(value, readCaret(), focused.value)}
                onSelect={() => pushState(value, readCaret(), focused.value)}
                onClick={() => pushState(value, readCaret(), focused.value)}
                onFocus={() => { focused.value = true; pushState(value, readCaret(), true); props.onFocus?.(); }}
                onBlur={() => { focused.value = false; pushState(value, readCaret(), false); props.onBlur?.(); }}
              />
              {showClear ? (
                <button
                  type="button"
                  class="field-clear"
                  onClick={() => commitProgrammaticValue('')}
                  aria-label={props.clearLabel ?? t(locale, 'clearField')}
                >
                  <Icon name="close" size={10} />
                </button>
              ) : null}
            </InputGroup>
          </FieldShell>
          {suggestMode ? (
            <AutocompletePopover
              anchor={(() => {
                const input = innerRef.value;
                if (!input) return null;
                const box = input.closest?.('.field__box');
                return (box as HTMLElement | null) ?? input;
              })()}
              open={snapshot.open}
              updateKey={cursor.value}
              anchorMode="field"
              onPopupPointerChange={(inside) => autocomplete.setPopupPointerInside(inside)}
            >
              <AutocompleteList
                sections={snapshot.sections}
                selectedIndex={snapshot.activeIndex}
                onHover={(index) => autocomplete.hover(index)}
                onPointerDown={() => autocomplete.beginPointerSelection()}
                onPointerUp={() => autocomplete.endPointerSelection()}
                onPick={(row) => {
                  const caret = readCaret();
                  const applied = autocomplete.pickRow(value, caret, row.key ?? row.item.value);
                  if (applied) applySuggestResult(applied);
                }}
                ariaLabel={props.label ?? 'Suggestions'}
                footer={t(locale, 'autocompleteNavigateInsert')}
                listId={autocomplete.listId}
                variant="compact"
              />
            </AutocompletePopover>
          ) : null}
        </>
      );
    };
  },
);

export default TextField;
</script>
