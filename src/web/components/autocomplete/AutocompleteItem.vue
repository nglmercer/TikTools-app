<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';
import { Icon, readIconName } from '../icons/index.ts';
import { highlightSegments } from './scoring.ts';
import { iconForSuggestion } from './autocomplete-controller.ts';
import type { SuggestionItem } from './types.ts';

export type AutocompleteItemProps = {
  /** Row content (S11: icon + label + badge + description). */
  item: SuggestionItem;
  /** Stable option id (`${listId}-option-${index}`). */
  id: string;
  selected: boolean;
  /** Highlight ranges into `item.value` from the fuzzy filter. */
  ranges?: Array<{ start: number; end: number }>;
  /** Hover/focus sync: point the active index at this row. */
  onHover: () => void;
  onPick: () => void;
  /** Row press started (pointerdown); lets the session win blur races. */
  onPointerDown?: () => void;
  /** Row press ended without committing (drag-off, cancel). */
  onPointerUp?: () => void;
};

/**
 * One dropdown row: semantic icon + label + subtle 9px badge + value path
 * and description lines. Icons are `IconName` only — `item.icon` from
 * JSON/plugins is whitelist-checked via `readIconName`, never raw SVG.
 */
export const AutocompleteItem = defineVueComponent<AutocompleteItemProps>(
  ['item', 'id', 'selected', 'ranges', 'onHover', 'onPick', 'onPointerDown', 'onPointerUp'],
  (props) => () => {
    const item = props.item;
    const iconName = (item.icon !== undefined ? readIconName(item.icon) : undefined) ?? iconForSuggestion(item);
    const showPath = item.value.length > 0 && item.value !== item.label;
    const firstLine = (item.description ?? '').split('\n')[0]?.trim() ?? '';
    const showDescription = firstLine.length > 0 && firstLine !== item.label && firstLine !== item.value;
    const hoverTitle = [item.value, item.detail ?? item.kind ? `type: ${item.detail ?? item.kind}` : '', item.preview ? `= ${item.preview}` : '', item.documentation ?? '']
      .filter(Boolean)
      .join('\n');
    const pathSegments = highlightSegments(item.value, props.ranges ?? []);
    return (
      <button
        type="button"
        role="option"
        id={props.id}
        tabindex={-1}
        aria-selected={props.selected}
        class={`autocomplete-item${props.selected ? ' is-selected' : ''}`}
        title={hoverTitle || item.value}
        onPointerdown={(event) => {
          // Touch/WebView: same rule as mouse — never steal input focus.
          event.preventDefault();
          props.onPointerDown?.();
        }}
        onMousedown={(event) => {
          event.preventDefault();
          props.onPointerDown?.();
        }}
        onMouseenter={props.onHover}
        onFocus={props.onHover}
        onPointerup={() => props.onPointerUp?.()}
        onPointercancel={() => props.onPointerUp?.()}
        onClick={props.onPick}
      >
        <span class="autocomplete-item__icon" aria-hidden="true">
          <Icon name={iconName} size={14} />
        </span>
        <span class="autocomplete-item__body">
          <span class="autocomplete-item__title">
            <span class="autocomplete-item__label">{item.label}</span>
            {item.badge ? <span class="autocomplete-item__badge">{item.badge}</span> : null}
          </span>
          {showPath ? (
            <code class="autocomplete-item__path">
              {pathSegments.map((segment, segmentIndex) => (
                segment.highlight
                  ? <mark key={segmentIndex}>{segment.text}</mark>
                  : <span key={segmentIndex}>{segment.text}</span>
              ))}
            </code>
          ) : null}
          {showDescription ? <span class="autocomplete-item__desc">{firstLine}</span> : null}
        </span>
      </button>
    );
  },
);

export default AutocompleteItem;
</script>
