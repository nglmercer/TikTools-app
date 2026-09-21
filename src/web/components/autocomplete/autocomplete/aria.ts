import { clampActiveIndex } from './keyboard.ts';

/* ------------------------------------------------------------------ */
/* ARIA (S15): combobox input attributes.                              */
/* ------------------------------------------------------------------ */

export type ComboboxInputAttrs = {
  role: 'combobox';
  'aria-expanded': boolean;
  'aria-controls': string | undefined;
  'aria-activedescendant': string | undefined;
  'aria-autocomplete': 'list';
  'aria-describedby': string | undefined;
  'aria-invalid': boolean;
  'aria-errormessage': string | undefined;
};

/**
 * Attributes Phase 3 spreads onto the text input. `listId` is the popup
 * `listbox` id; `activeIndex` selects the `aria-activedescendant` option
 * while open. `describedBy` should already include help/error ids.
 */
export function comboboxInputAttrs(input: {
  listId: string;
  open: boolean;
  activeIndex: number;
  rowCount: number;
  describedBy?: string;
  invalid?: boolean;
  errorId?: string;
}): ComboboxInputAttrs {
  const hasActive = input.open && input.rowCount > 0 && input.activeIndex >= 0;
  return {
    role: 'combobox',
    'aria-expanded': input.open,
    'aria-controls': input.open ? input.listId : undefined,
    'aria-activedescendant': hasActive ? `${input.listId}-option-${clampActiveIndex(input.activeIndex, input.rowCount)}` : undefined,
    'aria-autocomplete': 'list',
    'aria-describedby': input.describedBy,
    'aria-invalid': input.invalid === true,
    'aria-errormessage': input.invalid === true ? input.errorId : undefined,
  };
}
