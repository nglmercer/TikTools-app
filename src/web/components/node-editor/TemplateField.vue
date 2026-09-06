<script lang="tsx">
import { computed, ref, watch } from 'vue';
import type { VNode } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { applyFetchUrlTemplate, type FetchUrlTemplate } from './template-suggestions.ts';
import type { AutocompleteItem } from '../autocomplete/index.ts';
import { filterSuggestions, moveAutocompleteSelection, normalizeAutocompleteItems } from '../autocomplete/index.ts';
import { getAutocompleteToken } from '../autocomplete/token.ts';
import { AutocompleteList } from '../autocomplete/AutocompleteList.vue';
import { AutocompletePortal } from './AutocompletePortal.vue';
import { InfoTip } from '../ui/InfoTip.vue';
import { dispatchControlEvent, normalizeControlString, syncNativeControlValue } from '../ui/control-events.ts';
import { t, type Locale } from '../../i18n.ts';

export type TemplateFieldSuggestion = AutocompleteItem;

type TemplateFieldProps = {
  value: string;
  onValueChange: (value: string) => void;
  suggestions: TemplateFieldSuggestion[];
  suggestionMode?: 'template' | 'path';
  placeholder?: string;
  multiline?: boolean;
  rows?: number;
  ariaLabel?: string;
  name?: string;
  /** Kept for compatibility; no longer rendered (see dropdown footer). */
  hintText?: string;
  /** MUI-style floating label. When set, label lives inside until focus/filled. */
  label?: string;
  /** Tooltip-only explanation (ⓘ) attached to the floating label. */
  hint?: string;
  /** Kept for compatibility; the `{{ }}` badge was removed in favor of inline highlight. */
  template?: boolean;
  templateHint?: string;
  locale?: Locale;
  /**
   * When false, the dropdown only opens inside `{{ }}` or via Ctrl+Space.
   * Used by URL inputs so typing a hostname (e.g. `localhost`) never pops
   * event-variable suggestions or hijacks Tab.
   */
  bareWordTrigger?: boolean;
  /**
   * Quick URL targets (e.g. `localhost:3000`) offered at the top of the same
   * autocomplete dropdown while typing a URL. Picking one swaps only the
   * origin, keeping the typed path/query. Pass together with
   * `bareWordTrigger={false}` so bare words match presets, not variables.
   */
  urlPresets?: FetchUrlTemplate[];
};

/** URL preset as an autocomplete row: matched by label or URL, applied raw. */
function toPresetItem(preset: FetchUrlTemplate): AutocompleteItem & { label: string; value: string; preset: FetchUrlTemplate } {
  return {
    value: preset.url,
    label: preset.label,
    kind: 'snippet',
    detail: 'preset',
    documentation: preset.hint ?? preset.url,
    preview: preset.label,
    preset,
  };
}

/**
 * Small, dependency-free template editor. The stored value stays the runtime
 * format (`{{ event.data.comment }}`); `{{ … }}` spans render in cyan behind
 * the text so variables read as variables everywhere.
 *
 * Autocomplete is deliberately quiet: it opens only while typing inside
 * `{{ }}`, while typing a variable-like word (2+ chars), or via Ctrl+Space.
 */
export const TemplateField = defineVueComponent<TemplateFieldProps>(
  [
    'value',
    'onValueChange',
    'suggestions',
    'suggestionMode',
    'placeholder',
    'multiline',
    'rows',
    'ariaLabel',
    'name',
    'hintText',
    'label',
    'hint',
    'template',
    'templateHint',
    'locale',
    'bareWordTrigger',
    'urlPresets',
  ],
  (props) => {
  const inputRef = ref<HTMLInputElement | HTMLTextAreaElement | null>(null);
  const fieldRef = ref<HTMLDivElement | null>(null);
  const focused = ref(false);
  const forcedOpen = ref(false);
  const value = computed(() => normalizeControlString(props.value));
  const cursor = ref(value.value.length);
  const suggestionIndex = ref(0);

  watch(value, (next) => {
    if (inputRef.value) syncNativeControlValue(inputRef.value, next);
    cursor.value = Math.min(cursor.value, next.length);
  });

  const commitProgrammaticValue = (nextValue: string): void => {
    const control = inputRef.value;
    if (control) {
      syncNativeControlValue(control, nextValue);
      dispatchControlEvent(control);
    }
    props.onValueChange(nextValue);
  };

  const updateCursor = (event: Event): void => {
    const target = event.currentTarget as HTMLInputElement | HTMLTextAreaElement;
    cursor.value = target.selectionStart ?? target.value.length;
  };

  const syncHighlightScroll = (event: Event): void => {
    const target = event.currentTarget as HTMLInputElement | HTMLTextAreaElement;
    const backdrop = target.parentElement?.querySelector<HTMLElement>(':scope > .tpl-highlight');
    if (backdrop) {
      backdrop.scrollTop = target.scrollTop;
      backdrop.scrollLeft = target.scrollLeft;
    }
  };

  const suggestionMode = computed(() => props.suggestionMode ?? 'template');
  const token = computed(() => getAutocompleteToken(value.value, cursor.value, suggestionMode.value));
  const items = computed(() => normalizeAutocompleteItems(props.suggestions));
  // URL mode: presets live in the same dropdown; bare words match presets
  // only, variables stay behind `{{ }}` / Ctrl+Space.
  const urlMode = computed(() => suggestionMode.value === 'template' && Boolean(props.urlPresets && props.urlPresets.length > 0));
  const presetItems = computed(() => (urlMode.value ? (props.urlPresets ?? []).map(toPresetItem) : []));
  const scoredPresets = computed(() => (
    urlMode.value && !token.value.inside ? filterSuggestions(presetItems.value, token.value.query, 8) : []
  ));
  const scored = computed(() => filterSuggestions(items.value, token.value.query, 7));
  const presetRows = computed(() => scoredPresets.value.map((entry) => ({
    key: `preset:${entry.item.preset.id}`,
    preset: entry.item.preset as FetchUrlTemplate,
    item: entry.item,
    ranges: entry.matchRanges,
  })));
  const variableRows = computed(() => scored.value.map((entry) => ({
    key: `var:${entry.item.value}`,
    preset: undefined as FetchUrlTemplate | undefined,
    item: entry.item,
    ranges: entry.matchRanges,
  })));
  const showPresetRows = computed(() => presetRows.value.length > 0 && (forcedOpen.value || token.value.query.length >= 1));
  const bareWordHit = computed(() => (
    suggestionMode.value === 'path'
      ? token.value.query.length >= 1
      : (props.bareWordTrigger ?? true) && token.value.query.length >= 2
  ));
  const wantsOpenVars = computed(() => forcedOpen.value || token.value.inside || (!urlMode.value && bareWordHit.value));
  const showVariableRows = computed(() => wantsOpenVars.value && variableRows.value.length > 0);
  const visible = computed(() => [
    ...(showPresetRows.value ? presetRows.value : []),
    ...(showVariableRows.value ? variableRows.value : []),
  ]);
  const showSuggestions = computed(() => focused.value && visible.value.length > 0);

  watch(() => [token.value.query, props.suggestions.length, presetItems.value.length], () => {
    suggestionIndex.value = 0;
  });

  watch(showSuggestions, (open) => {
    if (!open) forcedOpen.value = false;
  });

  const insertSuggestion = (suggestion: { value: string }): void => {
    const element = inputRef.value;
    const offset = element?.selectionStart ?? cursor.value;
    const current = getAutocompleteToken(value.value, offset, suggestionMode.value);
    // Replace the exact range being typed: the `{{ …` span when inside
    // braces, the bare word (`event.us`) when completing outside them.
    // Inserting at the cursor without replacing duplicated the word.
    const start = suggestionMode.value === 'path' || current.inside || current.query.length > 0 ? current.start : offset;
    const inserted = suggestionMode.value === 'path' ? suggestion.value : `{{ ${suggestion.value} }}`;
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

  /** URL preset pick: swap only the origin, keeping the typed path/query. */
  const insertPreset = (preset: FetchUrlTemplate): void => {
    const nextValue = applyFetchUrlTemplate(value.value, preset.url);
    const nextCursor = nextValue.length;
    commitProgrammaticValue(nextValue);
    cursor.value = nextCursor;
    forcedOpen.value = false;
    requestAnimationFrame(() => {
      inputRef.value?.focus();
      inputRef.value?.setSelectionRange(nextCursor, nextCursor);
    });
  };

  const handleSuggestionKeydown = (event: KeyboardEvent): void => {
    const modifier = event.ctrlKey || event.metaKey;
    if (modifier && event.key === ' ') {
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
    } else if (event.key === 'Tab' || event.key === 'Enter') {
      // Enter on multiline without a query inserts a newline; Tab always picks.
      if (event.key === 'Enter' && (props.multiline ?? false) && !token.value.inside && !forcedOpen.value && token.value.query.length < 2) return;
      event.preventDefault();
      const selected = visible.value[suggestionIndex.value];
      if (selected?.preset) insertPreset(selected.preset);
      else if (selected) insertSuggestion(selected.item);
    } else if (event.key === 'Escape') {
      event.preventDefault();
      if (forcedOpen.value) forcedOpen.value = false;
      else focused.value = false;
      (event.currentTarget as HTMLElement).blur?.();
    }
  };

  return () => {
  const multiline = props.multiline ?? false;
  const rows = props.rows ?? 4;
  const locale = props.locale ?? 'en';
  const shared = {
    'aria-label': props.ariaLabel,
    spellcheck: false,
    onFocus: () => { focused.value = true; },
    onBlur: () => {
      focused.value = false;
      forcedOpen.value = false;
    },
    onKeydown: handleSuggestionKeydown,
    onInput: (event: Event) => {
      const target = event.currentTarget as HTMLInputElement | HTMLTextAreaElement;
      props.onValueChange(target.value);
      updateCursor(event);
    },
    onKeyup: updateCursor,
    onSelect: updateCursor,
    onClick: updateCursor,
    onScroll: syncHighlightScroll,
  } as const;

  const highlight = renderHighlight(value.value);

  const control = multiline ? (
    <textarea
      ref={(element) => { inputRef.value = element as HTMLTextAreaElement | null; }}
      class="node-editor-template-control node-editor-template-control--textarea tpl-transparent"
      name={props.name}
      value={value.value}
      rows={rows}
      placeholder={props.label ? ' ' : props.placeholder}
      {...shared}
    />
  ) : (
    <input
      ref={(element) => { inputRef.value = element as HTMLInputElement | null; }}
      class="node-editor-template-control tpl-transparent"
      type="text"
      name={props.name}
      value={value.value}
      placeholder={props.label ? ' ' : props.placeholder}
      {...shared}
    />
  );

  const presetCount = showPresetRows.value ? presetRows.value.length : 0;
  const dropdown = (
    <AutocompletePortal anchorRef={fieldRef} cursorRef={inputRef} cursorOffset={cursor.value} open={showSuggestions.value}>
      <AutocompleteList
        rows={visible.value.map(({ key, preset, item, ranges }) => ({ key, item, ranges, meta: preset }))}
        selectedIndex={suggestionIndex.value}
        onHover={(index) => { suggestionIndex.value = index; }}
        onPick={(row) => {
          const preset = row.meta as FetchUrlTemplate | undefined;
          if (preset) insertPreset(preset);
          else insertSuggestion(row.item);
        }}
        ariaLabel={props.ariaLabel ?? props.label ?? 'Suggestions'}
        groupLabel={presetCount > 0 ? t(locale, 'behavior.editor.urlPresets') : undefined}
        footer={t(locale, 'autocompleteNavigateInsert')}
      />
    </AutocompletePortal>
  );

  if (props.label) {
    const filled = value.value.trim().length > 0;
    return (
      <div ref={fieldRef} class={`node-editor-template-field node-editor-template-field--float ${filled ? 'is-filled' : ''} ${multiline ? 'is-multiline' : ''}`}>
        <div class="node-editor-template-control-wrap">
          <div class={`tpl-edit${multiline ? ' tpl-edit--multi' : ''}`}>
            <div class="tpl-highlight" aria-hidden="true">{highlight}</div>
            {control}
          </div>
          <label class="node-editor-float-label">
            <span class="node-editor-float-label__text">{props.label}</span>
            {props.hint ? <InfoTip text={props.hint} position="right" /> : null}
          </label>
        </div>
        {dropdown}
      </div>
    );
  }

  return (
    <div ref={fieldRef} class="node-editor-template-field">
      <div class="node-editor-template-control-wrap">
        <div class={`tpl-edit${multiline ? ' tpl-edit--multi' : ''}`}>
          <div class="tpl-highlight" aria-hidden="true">{highlight}</div>
          {control}
        </div>
      </div>
      {dropdown}
    </div>
  );
  };
  },
);

/** Inline `{{ … }}` highlight, including the unclosed `{{ …` state while typing. */
function renderHighlight(value: string): VNode[] | string {
  if (!value) return '';
  const parts = value.split(/(\{\{\s*[^}{]*\}?\}?)/g);
  return parts.map((part, index) => {
    if (index % 2 === 1) return <span key={index} class="tpl-var">{part || '{{'}</span>;
    return <span key={index}>{part}</span>;
  });
}

export default TemplateField;
</script>
