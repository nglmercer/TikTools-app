<script lang="tsx">
import { computed, ref, watch } from 'vue';
import type { VNode } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';
import { filterSuggestions, moveAutocompleteSelection, normalizeAutocompleteItems } from '../autocomplete/index.ts';
import { getAutocompleteToken } from '../autocomplete/token.ts';
import { AutocompleteList } from '../autocomplete/AutocompleteList.vue';
import { AutocompletePortal } from '../node-editor/AutocompletePortal.vue';
import { t, type Locale } from '../../i18n.ts';
import { formatJsonText, tokenizeJson } from './code-editor-logic.ts';
import { dispatchControlEvent, normalizeControlString, syncNativeControlValue } from './control-events.ts';

export { formatJsonText, tokenizeJson };

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
};

/**
 * Small dependency-free code editor: gutter with line numbers, a highlighted
 * backdrop (JSON tokens + `{{ }}` pills) behind a transparent textarea, and
 * the same quiet variable autocomplete as the single-line template inputs.
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
  ],
  (props) => {
  const inputRef = ref<HTMLTextAreaElement | null>(null);
  const boxRef = ref<HTMLDivElement | null>(null);
  const gutterRef = ref<HTMLDivElement | null>(null);
  const backdropRef = ref<HTMLPreElement | null>(null);
  const focused = ref(false);
  const forcedOpen = ref(false);
  const value = computed(() => normalizeControlString(props.value));
  const cursor = ref(value.value.length);
  const suggestionIndex = ref(0);

  const lineCount = computed(() => Math.max(1, value.value.split('\n').length));
  const lineHeight = 19.2; // 12px mono * 1.6
  const visibleLines = computed(() => Math.max(lineCount.value, props.rows ?? 7));
  const minEditHeight = computed(() => visibleLines.value * lineHeight + 20);
  const nodes = computed(() => (
    (props.language ?? 'text') === 'json' ? highlightJson(value.value) : highlightText(value.value)
  ));

  const token = computed(() => getAutocompleteToken(value.value, cursor.value));
  const items = computed(() => normalizeAutocompleteItems(props.suggestions ?? []));
  const scored = computed(() => filterSuggestions(items.value, token.value.query, 7));
  const visible = computed(() => scored.value.map((entry) => ({ item: entry.item, ranges: entry.matchRanges })));
  const showSuggestions = computed(() => (
    focused.value
    && (forcedOpen.value || token.value.inside || token.value.query.length >= 2)
    && visible.value.length > 0
  ));

  watch(() => [token.value.query, (props.suggestions ?? []).length], () => {
    suggestionIndex.value = 0;
  });

  watch(showSuggestions, (open) => {
    if (!open) forcedOpen.value = false;
  });

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

  const commitProgrammaticValue = (nextValue: string): void => {
    const control = inputRef.value;
    if (control) {
      syncNativeControlValue(control, nextValue);
      dispatchControlEvent(control);
    }
    props.onValueChange(nextValue);
  };

  const insertSuggestion = (suggestion: { value: string }): void => {
    const offset = inputRef.value?.selectionStart ?? cursor.value;
    const current = getAutocompleteToken(value.value, offset);
    // Replace the exact word/`{{ …` span being typed; inserting at the
    // cursor without replacing duplicated the typed text.
    const start = current.inside || current.query.length > 0 ? current.start : offset;
    const inserted = `{{ ${suggestion.value} }}`;
    const nextValue = `${value.value.slice(0, start)}${inserted}${value.value.slice(offset)}`;
    const nextCursor = start + inserted.length;
    commitProgrammaticValue(nextValue);
    cursor.value = nextCursor;
    forcedOpen.value = false;
    requestAnimationFrame(() => {
      inputRef.value?.focus();
      inputRef.value?.setSelectionRange(nextCursor, nextCursor);
    });
  };

  const handleKeyDown = (event: KeyboardEvent): void => {
    if ((event.ctrlKey || event.metaKey) && event.key === ' ') {
      event.preventDefault();
      forcedOpen.value = true;
      focused.value = true;
      return;
    }
    if (!showSuggestions.value || visible.value.length === 0) return;
    if (event.key === 'ArrowDown') {
      event.preventDefault();
      suggestionIndex.value = moveAutocompleteSelection(suggestionIndex.value, 1, visible.value.length);
    } else if (event.key === 'ArrowUp') {
      event.preventDefault();
      suggestionIndex.value = moveAutocompleteSelection(suggestionIndex.value, -1, visible.value.length);
    } else if (event.key === 'Tab' || (event.key === 'Enter' && (token.value.inside || forcedOpen.value))) {
      event.preventDefault();
      const selected = visible.value[suggestionIndex.value]?.item;
      if (selected) insertSuggestion(selected);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      if (forcedOpen.value) forcedOpen.value = false;
      else focused.value = false;
      (event.currentTarget as HTMLElement).blur?.();
    }
  };

  return () => {
    const rows = props.rows ?? 7;
    const locale = props.locale ?? 'en';
    const showHead = Boolean(props.filename || props.mime || props.onFormat);
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
            onFocus={() => { focused.value = true; }}
            onBlur={() => {
              focused.value = false;
              forcedOpen.value = false;
            }}
            onKeydown={handleKeyDown}
            onInput={(event) => {
              const target = event.currentTarget as HTMLTextAreaElement;
              props.onValueChange(target.value);
              updateCursor();
            }}
            onKeyup={updateCursor}
            onSelect={updateCursor}
            onClick={updateCursor}
            onScroll={syncScroll}
          />
        </div>
      </div>
      <AutocompletePortal anchorRef={boxRef} cursorRef={inputRef} cursorOffset={cursor.value} open={showSuggestions.value}>
        <AutocompleteList
          rows={visible.value.map(({ item, ranges }) => ({ item, ranges }))}
          selectedIndex={suggestionIndex.value}
          onHover={(index) => { suggestionIndex.value = index; }}
          onPick={(row) => insertSuggestion(row.item)}
          ariaLabel={props.ariaLabel ?? 'Suggestions'}
          footer={t(locale, 'autocompleteNavigateInsert')}
        />
      </AutocompletePortal>
    </div>
  );
  };
  },
);

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
