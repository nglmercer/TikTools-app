<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';
import { AutocompleteItem } from './AutocompleteItem.vue';
import { suggestionOptionId, type SuggestionRow } from './types.ts';

export type AutocompleteSectionProps = {
  /** Section id (used for the group label id). */
  id: string;
  /** Visible group label. Empty labels render rows without a header. */
  label: string;
  rows: SuggestionRow[];
  /** Global active index across all sections (hover/keyboard sync). */
  selectedIndex: number;
  /** Global index of `rows[0]` within the whole listbox. */
  startIndex: number;
  /** Listbox id; option ids derive as `${listId}-option-${index}`. */
  listId: string;
  onHover: (globalIndex: number) => void;
  onPick: (row: SuggestionRow) => void;
};

/**
 * One dropdown group (`role="group"`): optional header + rows. Indices
 * stay global across sections so keyboard/hover/aria-activedescendant
 * share one coordinate space with the controller.
 */
export const AutocompleteSection = defineVueComponent<AutocompleteSectionProps>(
  ['id', 'label', 'rows', 'selectedIndex', 'startIndex', 'listId', 'onHover', 'onPick'],
  (props) => () => {
    const labelId = `${props.listId}-${props.id}-label`;
    return (
      <div role="group" aria-labelledby={props.label ? labelId : undefined} class="autocomplete-section">
        {props.label ? <div id={labelId} class="autocomplete-section__label">{props.label}</div> : null}
        {props.rows.map((row, index) => {
          const globalIndex = props.startIndex + index;
          return (
            <AutocompleteItem
              key={row.key}
              id={suggestionOptionId(props.listId, globalIndex)}
              item={row.item}
              ranges={row.ranges}
              selected={globalIndex === props.selectedIndex}
              onHover={() => props.onHover(globalIndex)}
              onPick={() => props.onPick(row)}
            />
          );
        })}
      </div>
    );
  },
);

export default AutocompleteSection;
</script>
