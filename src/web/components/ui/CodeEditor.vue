<script lang="tsx">
import { computed, ref, watch } from 'vue';
import type { VNode } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';
import { comboboxInputAttrs } from '../autocomplete/autocomplete-controller.ts';
import { useAutocompleteInput } from '../autocomplete/use-autocomplete.ts';
import { AutocompleteList } from '../autocomplete/AutocompleteList.vue';
import { AutocompletePopover } from '../autocomplete/AutocompletePopover.vue';
import { t, type Locale } from '../../i18n.ts';
import { formatJsonText, shouldFormatPastedJson, tokenizeJson, validateJsonText, type JsonValidation } from './code-editor-logic.ts';
import { IconCheck, IconWarning } from '../icons/index.ts';
import { dispatchControlEvent, normalizeControlString, syncNativeControlValue } from './control-events.ts';

export { formatJsonText, shouldFormatPastedJson, tokenizeJson, validateJsonText, type JsonValidation };

export type CodeEditorLanguage = 'json' | 'text';

type CodeEditorProps = {
  value: string;
  onValueChange: (value: string) => void;
  /** Variable suggestions for `{{ }}` autocomplete. */
  suggestions?: AutocompleteItem[];
  language?: CodeEditorLanguage;
  locale?: Locale;
  /** File tab shown in the header, e.g. `payload.json`. */
  filename?: string;
  /** Mime shown in the header, e.g. `application/json`. */
  mime?: string;
  rows?: number;
  ariaLabel?: string;
  name?: string;
  /** When set, a format button renders in the header. */
  onFormat?: () => void;
  formatLabel?: string;
  /** Continuous JSON validation; defaults to true for `language="json"`. */
  validateJson?: boolean;
  /** Auto-format valid JSON pasted over an empty/fully-selected document. */
  formatJsonOnPaste?: boolean;
  /** Show the valid/invalid status line; defaults to true. */
  showValidationStatus?: boolean;
};

/**
 * Small dependency-free code editor: gutter with line numbers, a highlighted
 * backdrop (JSON tokens + `{{ }}` pills) behind a transparent textarea, and
 * template-variable autocomplete through the shared controller (opens inside
 * `{{ }}` or via Ctrl+Space, like TemplateField).
 */
export const CodeEditor = defineVueComponent<CodeEditorProps>(
  [
    'value',
    'onValueChange',
    'suggestions',
    'language',
    'locale',
    'filename',
    'mime',
    'rows',
    'ariaLabel',
    'name',
    'onFormat',
    'formatLabel',
    'validateJson',
    'formatJsonOnPaste',
    'showValidationStatus',
  ],
  (props) => {
  const inputRef = ref<HTMLTextAreaElement | null>(null);
  const boxRef = ref<HTMLDivElement | null>(null);
  const gutterRef = ref<HTMLDivElement | null>(null);
  const backdropRef = ref<HTMLPreElement | null>(null);
  const focused = ref(false);
  const value = computed(() => normalizeControlString(props.value));
  const cursor = ref(value.value.length);
  const autocomplete = useAutocompleteInput(() => ({
    mode: 'template',
    suggestions: props.suggestions,
    supportsTemplates: true,
    locale: props.locale ?? 'en',
  }));
  const language = computed(() => props.language ?? 'text');
  const validateJsonEnabled = computed(() => props.validateJson ?? (language.value === 'json'));
  const validation = computed<JsonValidation | null>(() => (
    language.value === 'json' && validateJsonEnabled.value ? validateJsonText(value.value) : null
  ));

  const lineCount = computed(() => Math.max(1, value.value.split('\n').length));
  const lineHeight = 19.2; // 12px mono * 1.6
  const visibleLines = computed(() => Math.max(lineCount.value, props.rows ?? 7));
  const minEditHeight = computed(() => visibleLines.value * lineHeight + 20);
  const nodes = computed(() => (
    language.value === 'json' ? highlightJson(value.value) : highlightText(value.value)
  ));

  watch(value, (next) => {
    if (inputRef.value) syncNativeControlValue(inputRef.value, next);
    cursor.value = Math.min(cursor.value, next.length);
  });

  const syncScroll = (): void => {
    const target = inputRef.value;
    if (!target) return;
    if (gutterRef.value) gutterRef.value.scrollTop = target.scrollTop;
    if (backdropRef.value) {
      backdropRef.value.scrollTop = target.scrollTop;
      backdropRef.value.scrollLeft = target.scrollLeft;
    }
  };

  const updateCursor = (): void => {
    const target = inputRef.value;
    cursor.value = target?.selectionStart ?? value.value.length;
  };

  const pushState = (next?: string): void => {
    updateCursor();
    autocomplete.update(next ?? value.value, cursor.value, focused.value);
  };

  const commitProgrammaticValue = (nextValue: string): void => {
    const control = inputRef.value;
    if (control) {
      syncNativeControlValue(control, nextValue);
      dispatchControlEvent(control);
    }
    props.onValueChange(nextValue);
  };

  const applyCommitResult = (nextValue: string, nextCursor: number): void => {
    commitProgrammaticValue(nextValue);
    cursor.value = nextCursor;
    requestAnimationFrame(() => {
      inputRef.value?.focus();
      inputRef.value?.setSelectionRange(nextCursor, nextCursor);
    });
  };

  const handlePaste = (event: ClipboardEvent): void => {
    const textarea = inputRef.value;
    const pasted = event.clipboardData?.getData('text/plain') ?? '';
    if (!shouldFormatPastedJson({
      language: language.value,
      validateJson: validateJsonEnabled.value,
      formatJsonOnPaste: props.formatJsonOnPaste ?? true,
      pastedText: pasted,
      currentValue: value.value,
      selectionStart: textarea?.selectionStart ?? 0,
      selectionEnd: textarea?.selectionEnd ?? 0,
    })) return;
    const formatted = formatJsonText(pasted);
    if (formatted === null) return;
    event.preventDefault();
    commitProgrammaticValue(formatted);
    cursor.value = formatted.length;
    requestAnimationFrame(() => {
      inputRef.value?.focus();
      inputRef.value?.setSelectionRange(formatted.length, formatted.length);
    });
  };

  const handleKeyDown = (event: KeyboardEvent): void => {
    const result = autocomplete.keydown(event);
    if (result !== 'commit') return;
    const offset = inputRef.value?.selectionStart ?? cursor.value;
    const applied = autocomplete.commit(value.value, offset);
    if (applied) applyCommitResult(applied.value, applied.caret);
  };

  return () => {
    const rows = props.rows ?? 7;
    const locale = props.locale ?? 'en';
    const showHead = Boolean(props.filename || props.mime || props.onFormat);
    const showStatus = (props.showValidationStatus ?? true) && validation.value !== null && validation.value.state !== 'empty';
    const snapshot = autocomplete.snapshot.value;
    const comboAttrs = comboboxInputAttrs({
      listId: autocomplete.listId,
      open: snapshot.open,
      activeIndex: snapshot.activeIndex,
      rowCount: snapshot.rowCount,
    });
    return (
    <div ref={boxRef} class="codeed">
      {showHead && (
        <div class="codeed-head">
          <span class="codeed-file">
            {props.filename && (
              <>
                <i class="codeed-dot" aria-hidden="true" />
                {props.filename}
              </>
            )}
          </span>
          <span class="codeed-side">
            {props.mime && <span class="codeed-mime">{props.mime}</span>}
            {props.onFormat && (
              <button type="button" class="codeed-format" onClick={props.onFormat}>
                {props.formatLabel ?? t(locale, 'behavior.editor.format')}
              </button>
            )}
          </span>
        </div>
      )}
      <div class="codeed-body">
        <div ref={gutterRef} class="codeed-gutter" aria-hidden="true">
          {Array.from({ length: visibleLines.value }, (_, index) => (
            <span key={index + 1}>{index + 1}</span>
          ))}
        </div>
        <div class="codeed-edit" style={{ minHeight: `${minEditHeight.value}px` }}>
          <pre ref={backdropRef} class="codeed-backdrop" aria-hidden="true">
            <code>
              {nodes.value}
              {value.value.endsWith('\n') ? '\n​' : ''}
            </code>
          </pre>
          <textarea
            ref={inputRef}
            class="codeed-input"
            name={props.name}
            value={value.value}
            rows={rows}
            spellcheck={false}
            wrap="off"
            aria-label={props.ariaLabel}
            {...comboAttrs}
            onFocus={() => { focused.value = true; pushState(); }}
            onBlur={() => {
              focused.value = false;
              pushState();
            }}
            onKeydown={handleKeyDown}
            onInput={(event) => {
              const target = event.currentTarget as HTMLTextAreaElement;
              props.onValueChange(target.value);
              pushState(target.value);
            }}
            onPaste={handlePaste}
            onKeyup={() => pushState()}
            onSelect={() => pushState()}
            onClick={() => pushState()}
            onScroll={syncScroll}
          />
        </div>
      </div>
      {showStatus ? <CodeEditorStatus locale={locale} validation={validation.value!} /> : null}
      <AutocompletePopover
        anchor={boxRef.value}
        open={snapshot.open}
        updateKey={cursor.value}
        anchorMode="caret"
        input={inputRef.value}
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
            const offset = inputRef.value?.selectionStart ?? cursor.value;
            const applied = autocomplete.pickRow(value.value, offset, row.key ?? row.item.value);
            if (applied) applyCommitResult(applied.value, applied.caret);
          }}
          ariaLabel={props.ariaLabel ?? 'Suggestions'}
          footer={t(locale, 'autocompleteNavigateInsert')}
          listId={autocomplete.listId}
        />
      </AutocompletePopover>
    </div>
  );
  };
  },
);

/** Small validity indicator below the editor; never blocks typing. */
function CodeEditorStatus({ locale, validation }: { locale: Locale; validation: JsonValidation }) {
  if (validation.state === 'empty') return null;
  if (validation.state === 'valid') {
    return (
      <div class="codeed-status is-valid" role="status">
        <IconCheck size={12} />
        <span>{t(locale, 'jsonValid')}</span>
      </div>
    );
  }
  return (
    <div class="codeed-status is-invalid" role="status">
      <IconWarning size={12} />
      <span class="codeed-status__text">
        <strong>{t(locale, 'jsonInvalid')}</strong>
        <span>{validation.message}</span>
      </span>
    </div>
  );
}

/** Plain text: only `{{ }}` spans get the pill treatment. */
function highlightText(value: string): VNode[] {
  if (!value) return [<span key="empty">{''}</span>];
  const parts = value.split(/(\{\{\s*[^{}]*\}?\}?)/g);
  return parts.map((part, index) =>
    index % 2 === 1 ? <span key={index} class="codeed-var">{part || '{{'}</span> : <span key={index}>{part}</span>,
  );
}

function highlightJson(value: string): VNode[] {
  if (!value) return [<span key="empty">{''}</span>];
  const out: VNode[] = [];
  const varPattern = /\{\{\s*[^{}]*\}?\}?/g;
  let last = 0;
  let key = 0;
  let match: RegExpExecArray | null;
  const pushChunk = (chunk: string): void => {
    for (const token of tokenizeJson(chunk)) out.push(<span key={key++} class={token.cls}>{token.text}</span>);
  };
  while ((match = varPattern.exec(value)) !== null) {
    if (match.index > last) pushChunk(value.slice(last, match.index));
    out.push(<span key={key++} class="codeed-var">{match[0]}</span>);
    last = match.index + match[0].length;
  }
  if (last < value.length) pushChunk(value.slice(last));
  return out;
}

export default CodeEditor;
</script>
