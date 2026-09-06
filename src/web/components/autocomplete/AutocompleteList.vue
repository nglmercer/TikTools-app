<script lang="tsx">
import { defineVueComponent } from '../../vue/component.ts';
import type { AutocompleteItem, AutocompleteRow } from './types.ts';
import { highlightSegments } from './scoring.ts';

export type AutocompleteListProps = {
  rows: AutocompleteRow[];
  selectedIndex: number;
  onHover: (index: number) => void;
  onPick: (row: AutocompleteRow) => void;
  ariaLabel?: string;
  groupLabel?: string;
  footer?: string;
};

export const AutocompleteList = defineVueComponent<AutocompleteListProps>(
  ['rows', 'selectedIndex', 'onHover', 'onPick', 'ariaLabel', 'groupLabel', 'footer'],
  (props) => () => (
    <div class="tpl-suggest" role="listbox" aria-label={props.ariaLabel ?? 'Suggestions'}>
      {props.groupLabel ? <div class="tpl-group">{props.groupLabel}</div> : null}
      {props.rows.map((row, index) => <AutocompleteRowButton
        key={row.key ?? `${row.item.value}:${index}`}
        row={row}
        selected={index === props.selectedIndex}
        onHover={() => props.onHover(index)}
        onPick={() => props.onPick(row)}
      />)}
      {props.footer ? <div class="tpl-foot">{props.footer}</div> : null}
    </div>
  ),
);

export default AutocompleteList;

function AutocompleteRowButton({
  row,
  selected,
  onHover,
  onPick,
}: {
  row: AutocompleteRow;
  selected: boolean;
  onHover: () => void;
  onPick: () => void;
}) {
  const item: AutocompleteItem = row.item;
  const valueSegments = highlightSegments(item.value, row.ranges);
  const hoverTitle = [
    item.value,
    item.detail ?? item.kind ? `type: ${item.detail ?? item.kind}` : '',
    item.preview ? `= ${item.preview}` : '',
    item.documentation ?? '',
  ].filter(Boolean).join('\n');
  return (
    <button
      type="button"
      role="option"
      aria-selected={selected}
      class={selected ? 'is-selected' : ''}
      title={hoverTitle || item.value}
      onMousedown={(event) => event.preventDefault()}
      onMouseenter={onHover}
      onFocus={onHover}
      onClick={onPick}
    >
      <i class={`tpl-dot tpl-dot--${item.kind ?? 'unknown'}`} aria-hidden="true" />
      <code class="tpl-path">
        {valueSegments.map((segment, segmentIndex) => (
          segment.highlight ? <mark key={segmentIndex}>{segment.text}</mark> : <span key={segmentIndex}>{segment.text}</span>
        ))}
      </code>
      {item.detail ?? item.kind ? <span class="tpl-kind">{item.detail ?? item.kind}</span> : null}
      {item.preview ? <span class="tpl-preview" title={item.preview}>{item.preview}</span> : null}
    </button>
  );
}
</script>
