import { AUTOCOMPLETE_PREFERRED_WIDTH, type AnchorRect } from './autocomplete-position.ts';

/**
 * Anchor strategies for the shared popover. `field` anchors to a whole
 * control box (URL presets, options lists, script completions); `caret`
 * anchors to a virtual zero-width rectangle at the text caret so template
 * and editor suggestions appear where the user is typing.
 */
export type AutocompleteAnchorMode = 'field' | 'caret';

/** Pick the anchor a popover placement should use. Caret mode falls back to the field box when no caret rect is available. */
export function resolveAnchorRect(input: {
  mode: AutocompleteAnchorMode;
  field: AnchorRect | null;
  caret: AnchorRect | null;
}): AnchorRect | null {
  if (input.mode === 'caret') return input.caret ?? input.field;
  return input.field;
}

/**
 * Desired popup width before viewport clamping. A caret anchor is a
 * zero-width point, so caret popovers fall back to the preferred width
 * instead of the anchor width; field popovers match their control box.
 * An explicit `preferredWidth` always wins.
 */
export function resolveDesiredPopupWidth(input: {
  anchorWidth: number;
  mode: AutocompleteAnchorMode;
  preferredWidth?: number;
}): number {
  if (typeof input.preferredWidth === 'number') return input.preferredWidth;
  if (input.mode === 'caret') return AUTOCOMPLETE_PREFERRED_WIDTH;
  return input.anchorWidth;
}

export type CaretMarkerMetrics = {
  /** Viewport position of the input box. */
  inputLeft: number;
  inputTop: number;
  /** Marker position inside the mirror, in mirror coordinates. */
  markerLeft: number;
  markerTop: number;
  markerHeight: number;
  /** Current scroll offsets of the real input. */
  scrollLeft: number;
  scrollTop: number;
};

/**
 * Convert mirror-marker coordinates into a viewport caret anchor: a thin
 * rectangle at the caret's line. Pure so bun tests pin the math; the DOM
 * helpers below only supply the metrics.
 */
export function caretAnchorFromMarker(metrics: CaretMarkerMetrics): AnchorRect {
  const left = metrics.inputLeft + metrics.markerLeft - metrics.scrollLeft;
  const top = metrics.inputTop + metrics.markerTop - metrics.scrollTop;
  const height = Math.max(1, metrics.markerHeight);
  return { top, left, bottom: top + height, width: 2 };
}

/** Typography + box properties a caret mirror must copy to match wrapping. */
const MIRROR_TEXT_PROPERTIES = [
  'fontFamily',
  'fontSize',
  'fontWeight',
  'fontStyle',
  'fontVariant',
  'letterSpacing',
  'textTransform',
  'textIndent',
  'lineHeight',
  'paddingTop',
  'paddingRight',
  'paddingBottom',
  'paddingLeft',
  'borderTopWidth',
  'borderRightWidth',
  'borderBottomWidth',
  'borderLeftWidth',
  'boxSizing',
  'tabSize',
] as const;

function copyMirrorTextStyle(source: HTMLElement, mirror: HTMLElement): void {
  const computed = window.getComputedStyle(source);
  for (const property of MIRROR_TEXT_PROPERTIES) {
    mirror.style.setProperty(
      property.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`),
      computed.getPropertyValue(property.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`)),
    );
  }
}

function measureMarker(
  input: HTMLInputElement | HTMLTextAreaElement,
  caret: number,
  multiline: boolean,
): { left: number; top: number; height: number } | null {
  const rect = input.getBoundingClientRect();
  const mirror = document.createElement('div');
  const style = mirror.style;
  style.position = 'fixed';
  style.visibility = 'hidden';
  style.pointerEvents = 'none';
  style.whiteSpace = multiline ? 'pre-wrap' : 'pre';
  style.wordWrap = multiline ? 'break-word' : 'normal';
  style.overflow = 'hidden';
  // Mirror the content box so wrapped lines break at the same columns.
  style.width = `${Math.max(0, rect.width)}px`;
  style.height = `${Math.max(0, rect.height)}px`;
  style.top = `${rect.top}px`;
  style.left = `${rect.left}px`;
  copyMirrorTextStyle(input, mirror);
  const safeCaret = Math.max(0, Math.min(caret, input.value.length));
  mirror.textContent = input.value.slice(0, safeCaret);
  const marker = document.createElement('span');
  // Zero-width mark: measurable without shifting the line.
  marker.textContent = '​';
  mirror.appendChild(marker);
  // Trailing content keeps the mirror's scroll geometry honest for inputs
  // scrolled past the caret.
  const tail = document.createElement('span');
  tail.textContent = input.value.slice(safeCaret) || '​';
  mirror.appendChild(tail);
  document.body.appendChild(mirror);
  const markerRect = marker.getBoundingClientRect();
  const mirrorRect = mirror.getBoundingClientRect();
  const metrics = {
    left: markerRect.left - mirrorRect.left,
    top: markerRect.top - mirrorRect.top,
    height: markerRect.height > 0 ? markerRect.height : parseFloat(window.getComputedStyle(input).lineHeight) || 16,
  };
  mirror.remove();
  return metrics;
}

/**
 * Viewport rectangle for the caret of a single-line input. Returns null
 * outside the DOM or when the caret cannot be read (some input types
 * throw on `selectionStart`).
 */
export function getInputCaretRect(input: HTMLInputElement, caret: number): AnchorRect | null {
  try {
    if (typeof document === 'undefined' || !document.body) return null;
    const rect = input.getBoundingClientRect();
    const marker = measureMarker(input, caret, false);
    if (!marker) return null;
    return caretAnchorFromMarker({
      inputLeft: rect.left,
      inputTop: rect.top,
      markerLeft: marker.left,
      markerTop: marker.top,
      markerHeight: marker.height,
      scrollLeft: input.scrollLeft,
      scrollTop: input.scrollTop,
    });
  } catch {
    return null;
  }
}

/** Viewport rectangle for the caret of a textarea (wrapping mirror). */
export function getTextareaCaretRect(input: HTMLTextAreaElement, caret: number): AnchorRect | null {
  try {
    if (typeof document === 'undefined' || !document.body) return null;
    const rect = input.getBoundingClientRect();
    const marker = measureMarker(input, caret, true);
    if (!marker) return null;
    return caretAnchorFromMarker({
      inputLeft: rect.left,
      inputTop: rect.top,
      markerLeft: marker.left,
      markerTop: marker.top,
      markerHeight: marker.height,
      scrollLeft: input.scrollLeft,
      scrollTop: input.scrollTop,
    });
  } catch {
    return null;
  }
}

/** Caret anchor for either control kind; null when it cannot be measured. */
export function getCaretAnchorRect(
  input: HTMLInputElement | HTMLTextAreaElement | null | undefined,
  caret: number,
): AnchorRect | null {
  if (!input) return null;
  if (input.tagName === 'TEXTAREA') {
    return getTextareaCaretRect(input as HTMLTextAreaElement, caret);
  }
  return getInputCaretRect(input as HTMLInputElement, caret);
}
