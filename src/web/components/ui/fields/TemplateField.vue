<script lang="tsx">
import { ref, watch } from 'vue';
import { defineVueComponent } from '../../../vue/component.ts';
import { t, type Locale } from '../../../i18n.ts';
import type { TooltipPosition } from '../tooltip-logic.ts';
import { dispatchControlEvent, normalizeControlString, syncNativeControlValue } from '../control-events.ts';
import {
  comboboxInputAttrs,
  type PresetItem,
} from '../../autocomplete/autocomplete-controller.ts';
import type { SuggestionItem } from '../../autocomplete/types.ts';
import { useAutocompleteInput } from '../../autocomplete/use-autocomplete.ts';
import { AutocompleteList } from '../../autocomplete/AutocompleteList.vue';
import { AutocompletePopover } from '../../autocomplete/AutocompletePopover.vue';
import { FieldShell } from './FieldShell.vue';
import { InputGroup } from './InputGroup.vue';
import { describeField, fieldControlId, fieldMessageIds, type FieldSize } from './field-logic.ts';

/** Template variable scope. The controller filters suggestions to it. */
export type TemplateScope = 'message' | 'identity' | 'http-url' | 'http-data' | 'generic';

export type TemplateFieldHandle = {
  getValue: () => string;
  setValue: (v: string) => void;
  focus: () => void;
  clear: () => void;
  validate: () => boolean;
};

export type TemplateFieldProps = {
  value: string;
  onValueChange: (v: string) => void;
  /** Variable scope for suggestions (default `generic`). */
  scope?: TemplateScope;
  /** Template variable pool (pre-scoped by the caller; untagged = universal). */
  suggestions?: SuggestionItem[];
  /**
   * URL presets offered outside `{{ }}` (Quick destinations, zero event
   * variables). Template mode only; ignored in options mode.
   */
  presets?: PresetItem[];
  /**
   * `template` (default): `{{ }}` variables via the template controller.
   * `options`: bare-value completion without braces (path-like inputs).
   */
  mode?: 'template' | 'options';
  multiline?: boolean;
  rows?: number;
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
  ariaLabel?: string;
  required?: boolean;
  disabled?: boolean;
  readonly?: boolean;
  size?: FieldSize;
  locale?: Locale;
  autoComplete?: string;
  spellCheck?: boolean;
  maxLength?: number;
  /** Single-line only: multiline Enter inserts a newline instead. */
  onEnter?: () => void;
  className?: string;
};

let templateFieldFallback = 0;

/**
 * Dedicated template input: the ONLY field wired to the template-mode
 * autocomplete controller. Generic TextField never suggests variables.
 * Opens inside `{{ }}` (or via Ctrl+Space), filters by `scope`, inserts
 * `{{ value }}` preserving surrounding text and caret. With `presets`,
 * Quick-destination presets show outside braces through the same dropdown.
 */
export const TemplateField = defineVueComponent<TemplateFieldProps>(
  ['value', 'onValueChange', 'scope', 'suggestions', 'presets', 'mode', 'multiline', 'rows', 'id', 'name', 'label', 'hint', 'hintPosition', 'description', 'error', 'placeholder', 'ariaLabel', 'required', 'disabled', 'readonly', 'size', 'locale', 'autoComplete', 'spellCheck', 'maxLength', 'onEnter', 'className'],
  (props, context) => {
    const innerRef = ref<HTMLInputElement | HTMLTextAreaElement | null>(null);
    const focused = ref(false);
    const cursor = ref(normalizeControlString(props.value).length);
    templateFieldFallback += 1;
    const fallbackId = `tt-template-${templateFieldFallback}`;
    const autocomplete = useAutocompleteInput(() => ({
      mode: props.mode ?? 'template',
      suggestions: props.suggestions,
      presets: props.presets,
      scope: props.scope ?? 'generic',
      supportsTemplates: true,
      locale: props.locale ?? 'en',
    }));

    const readCaret = (): number => {
      const control = innerRef.value;
      return control?.selectionStart ?? normalizeControlString(props.value).length;
    };

    const applyResult = (value: string, caret: number): void => {
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

    context.expose({
      getValue: () => innerRef.value?.value ?? normalizeControlString(props.value),
      setValue: commitProgrammaticValue,
      focus: () => innerRef.value?.focus(),
      clear: () => commitProgrammaticValue(''),
      validate: () => !(props.required && !normalizeControlString(props.value).trim()),
    } satisfies TemplateFieldHandle);

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
      const multiline = props.multiline ?? false;
      const snapshot = autocomplete.snapshot.value;
      const comboAttrs = comboboxInputAttrs({
        listId: autocomplete.listId,
        open: snapshot.open,
        activeIndex: snapshot.activeIndex,
        rowCount: snapshot.rowCount,
        describedBy: describeField([props.description ? descriptionId : undefined, props.error ? errorId : undefined]),
        invalid,
        errorId: props.error ? errorId : undefined,
      });
      const shared = {
        id,
        'aria-label': props.ariaLabel,
        ...comboAttrs,
      } as const;
      const control = multiline ? (
        <textarea
          ref={(element) => { innerRef.value = element as HTMLTextAreaElement | null; }}
          name={props.name}
          value={value}
          rows={props.rows ?? 4}
          placeholder={props.placeholder}
          disabled={props.disabled}
          readonly={props.readonly}
          autocomplete={props.autoComplete ?? 'off'}
          spellcheck={props.spellCheck ?? false}
          required={props.required}
          maxlength={props.maxLength}
          class="field-input field-input--textarea"
          {...shared}
          onInput={(event) => {
            const target = event.currentTarget as HTMLTextAreaElement;
            props.onValueChange(target.value);
            cursor.value = target.selectionStart ?? target.value.length;
            autocomplete.update(target.value, cursor.value, true);
          }}
          onKeydown={(event) => {
            const result = autocomplete.keydown(event);
            if (result === 'commit') {
              const caret = readCaret();
              const applied = autocomplete.commit(value, caret);
              if (applied) applyResult(applied.value, applied.caret);
            }
          }}
          onKeyup={() => { cursor.value = readCaret(); autocomplete.update(value, cursor.value, focused.value); }}
          onSelect={() => { cursor.value = readCaret(); autocomplete.update(value, cursor.value, focused.value); }}
          onClick={() => { cursor.value = readCaret(); autocomplete.update(value, cursor.value, focused.value); }}
          onFocus={() => { focused.value = true; autocomplete.update(value, readCaret(), true); }}
          onBlur={() => { focused.value = false; autocomplete.update(value, readCaret(), false); }}
        />
      ) : (
        <input
          ref={(element) => { innerRef.value = element as HTMLInputElement | null; }}
          name={props.name}
          type="text"
          value={value}
          placeholder={props.placeholder}
          disabled={props.disabled}
          readonly={props.readonly}
          autocomplete={props.autoComplete ?? 'off'}
          spellcheck={props.spellCheck ?? false}
          required={props.required}
          maxlength={props.maxLength}
          class="field-input"
          {...shared}
          onInput={(event) => {
            const target = event.currentTarget as HTMLInputElement;
            props.onValueChange(target.value);
            cursor.value = target.selectionStart ?? target.value.length;
            autocomplete.update(target.value, cursor.value, true);
          }}
          onKeydown={(event) => {
            const result = autocomplete.keydown(event);
            if (result === 'commit') {
              const caret = readCaret();
              const applied = autocomplete.commit(value, caret);
              if (applied) applyResult(applied.value, applied.caret);
            } else if (event.key === 'Enter' && props.onEnter) {
              props.onEnter();
            }
          }}
          onKeyup={() => { cursor.value = readCaret(); autocomplete.update(value, cursor.value, focused.value); }}
          onSelect={() => { cursor.value = readCaret(); autocomplete.update(value, cursor.value, focused.value); }}
          onClick={() => { cursor.value = readCaret(); autocomplete.update(value, cursor.value, focused.value); }}
          onFocus={() => { focused.value = true; autocomplete.update(value, readCaret(), true); }}
          onBlur={() => { focused.value = false; autocomplete.update(value, readCaret(), false); }}
        />
      );
      return (
        <div class={`field-template${multiline ? ' field-template--multiline' : ''}`} data-template-scope={props.scope ?? 'generic'}>
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
              focused={focused.value}
              invalid={invalid}
              disabled={props.disabled}
            >
              {control}
            </InputGroup>
          </FieldShell>
          <AutocompletePopover
            anchor={innerRef.value}
            open={snapshot.open}
            updateKey={cursor.value}
            anchorMode="caret"
            input={innerRef.value}
            caretOffset={cursor.value}
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
                if (applied) applyResult(applied.value, applied.caret);
              }}
              ariaLabel={props.ariaLabel ?? props.label ?? 'Suggestions'}
              footer={t(locale, 'autocompleteNavigateInsert')}
              listId={autocomplete.listId}
            />
          </AutocompletePopover>
        </div>
      );
    };
  },
);

export default TemplateField;
</script>
