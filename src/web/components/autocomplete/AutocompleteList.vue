<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';
import type { AutocompleteRow, SuggestionRow, SuggestionSection } from './types.ts';
import { AutocompleteItem } from './AutocompleteItem.vue';
import { AutocompleteSection } from './AutocompleteSection.vue';
import { suggestionOptionId } from './types.ts';

export type AutocompleteListDensity = 'comfortable' | 'compact';

export type AutocompleteListProps = {
  /** Legacy flat rows (TemplateField/CodeEditor pass these; `meta` passes through to `onPick`). */
  rows?: AutocompleteRow[];
  /** Grouped rows (new callers pass these; preferred over `rows`). */
  sections?: SuggestionSection[];
  /** Global active index across sections. */
  selectedIndex: number;
  onHover: (index: number) => void;
  onPick: (row: AutocompleteRow) => void;
  /** Row press started (pointerdown); lets the session win blur races. */
  onPointerDown?: () => void;
  /** Row press ended without committing (drag-off, cancel). */
  onPointerUp?: () => void;
  ariaLabel?: string;
  groupLabel?: string;
  footer?: string;
  /** Listbox id; defaults to a stable per-instance id. Drives `aria-activedescendant`. */
  listId?: string;
  /**
   * `compact` renders the same rows/selection as a small editor hint
   * (tiny section captions, thin status footer) for single-purpose
   * popups such as URL presets. Default `comfortable`.
   */
  variant?: AutocompleteListDensity;
};

let autocompleteListFallback = 0;

/**
 * Listbox popup content: sections (or legacy flat rows) of icon + label +
 * badge + description rows. `listId` + `selectedIndex` give Phase 3 the
 * `aria-activedescendant` target (`${listId}-option-${selectedIndex}`).
 */
export const AutocompleteList = defineVueComponent<AutocompleteListProps>(
  ['rows', 'selectedIndex', 'onHover', 'onPick', 'onPointerDown', 'onPointerUp', 'ariaLabel', 'groupLabel', 'footer', 'sections', 'listId', 'variant'],
  (props) => {
    autocompleteListFallback += 1;
    const fallbackId = `tt-ac-list-${autocompleteListFallback}`;
    return () => {
      const listId = props.listId ?? fallbackId;
      const sections = props.sections;
      const compact = (props.variant ?? 'comfortable') === 'compact';
      return (
        <div id={listId} class={`autocomplete-list${compact ? ' autocomplete-list--compact' : ''}`} role="listbox" aria-label={props.ariaLabel ?? 'Suggestions'}>
          {sections && sections.length > 0 ? (
            <SectionedRows
              sections={sections}
              listId={listId}
              selectedIndex={props.selectedIndex}
              onHover={props.onHover}
              onPick={props.onPick}
              onPointerDown={props.onPointerDown}
              onPointerUp={props.onPointerUp}
            />
          ) : (
            <>
              {props.groupLabel ? <div class="autocomplete-section__label">{props.groupLabel}</div> : null}
              {(props.rows ?? []).map((row, index) => (
                <AutocompleteItem
                  key={row.key ?? `${row.item.value}:${index}`}
                  id={suggestionOptionId(listId, index)}
                  item={row.item}
                  ranges={row.ranges}
                  selected={index === props.selectedIndex}
                  onHover={() => props.onHover(index)}
                  onPick={() => props.onPick(row)}
                  onPointerDown={props.onPointerDown}
                  onPointerUp={props.onPointerUp}
                />
              ))}
            </>
          )}
          {props.footer ? <div class="autocomplete-list__footer">{props.footer}</div> : null}
        </div>
      );
    };
  },
);

export default AutocompleteList;

function SectionedRows({
  sections,
  listId,
  selectedIndex,
  onHover,
  onPick,
  onPointerDown,
  onPointerUp,
}: {
  sections: SuggestionSection[];
  listId: string;
  selectedIndex: number;
  onHover: (index: number) => void;
  onPick: (row: AutocompleteRow) => void;
  onPointerDown?: () => void;
  onPointerUp?: () => void;
}) {
  let startIndex = 0;
  return (
    <>
      {sections.map((section) => {
        const start = startIndex;
        startIndex += section.rows.length;
        return (
          <AutocompleteSection
            key={section.id}
            id={section.id}
            label={section.label}
            rows={section.rows}
            selectedIndex={selectedIndex}
            startIndex={start}
            listId={listId}
            onHover={onHover}
            onPick={(row: SuggestionRow) => onPick(row)}
            onPointerDown={onPointerDown}
            onPointerUp={onPointerUp}
          />
        );
      })}
    </>
  );
}
</script>
