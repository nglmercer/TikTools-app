/* ------------------------------------------------------------------ */
/* Keyboard (S14): Up/Down/Enter/Escape/Tab/Home/End + hover sync.      */
/* ------------------------------------------------------------------ */

export type AutocompleteKeyAction = 'next' | 'prev' | 'first' | 'last' | 'commit' | 'dismiss';

/**
 * Map one key to its listbox action, or null when the key is not handled.
 * Up/Down move (wrapping), Home/End jump, Enter/Tab commit, Escape
 * dismisses. Escape closes WITHOUT moving focus — focus stays on the
 * input (S15); the controller never blurs.
 */
export function resolveAutocompleteKey(key: string): AutocompleteKeyAction | null {
  switch (key) {
    case 'ArrowDown': return 'next';
    case 'ArrowUp': return 'prev';
    case 'Home': return 'first';
    case 'End': return 'last';
    case 'Enter':
    case 'Tab': return 'commit';
    case 'Escape': return 'dismiss';
    default: return null;
  }
}

/** Ctrl/Meta+Space explicitly invokes the dropdown from anywhere. */
export function isExplicitInvokeKey(event: { key: string; ctrlKey: boolean; metaKey: boolean }): boolean {
  return event.key === ' ' && (event.ctrlKey || event.metaKey);
}

/** Wrap-around step for Up/Down. Empty lists stay at 0. */
export function moveActiveIndex(index: number, delta: number, length: number): number {
  if (length <= 0) return 0;
  return (index + delta + length) % length;
}

/** Clamp any index (hover sync, Home/End) into range. */
export function clampActiveIndex(index: number, length: number): number {
  if (length <= 0) return 0;
  return Math.max(0, Math.min(index, length - 1));
}
