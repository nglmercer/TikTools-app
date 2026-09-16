export type ModalSize = 'sm' | 'md' | 'lg' | 'xl';

/** Width class suffix per modal size; `md` is the default 440px card. */
export const MODAL_SIZE_CLASS: Record<ModalSize, string> = {
  sm: 'ui-modal-card--sm',
  md: '',
  lg: 'ui-modal-card--lg',
  xl: 'ui-modal-card--xl',
};

/** Focusable elements cycled by the modal focus trap. */
export const FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled])',
  'textarea:not([disabled])',
  'select:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(', ');

let modalSequence = 0;

export type ModalIds = {
  titleId: string;
  descriptionId: string;
};

/**
 * Instance-specific accessible ids. Every modal gets its own `aria-labelledby`
 * target instead of sharing one hardcoded id across the document.
 */
export function createModalIds(prefix = 'ui-modal'): ModalIds {
  modalSequence += 1;
  return {
    titleId: `${prefix}-title-${modalSequence}`,
    descriptionId: `${prefix}-description-${modalSequence}`,
  };
}

/**
 * Focus-trap step for Tab / Shift+Tab. Returns the index to move focus to when
 * the keypress would leave the dialog, or `null` to let the browser move
 * focus natively. `focusedIndex` is the position of `document.activeElement`
 * within the dialog's focusable elements (`-1` when focus is outside).
 */
export function trapFocusTarget(
  focusedIndex: number,
  focusableCount: number,
  shiftKey: boolean,
): number | null {
  if (focusableCount <= 0) return null;
  if (shiftKey) {
    if (focusedIndex <= 0) return focusableCount - 1;
    return null;
  }
  if (focusedIndex < 0 || focusedIndex >= focusableCount - 1) return 0;
  return null;
}

/** Backdrop clicks close only when enabled and the backdrop itself was hit. */
export function isBackdropDismiss(
  closeOnBackdrop: boolean,
  target: unknown,
  currentTarget: unknown,
): boolean {
  return closeOnBackdrop && target === currentTarget;
}

/** Escape dismisses only when enabled. */
export function isEscapeDismiss(closeOnEscape: boolean, key: string): boolean {
  return closeOnEscape && key === 'Escape';
}
