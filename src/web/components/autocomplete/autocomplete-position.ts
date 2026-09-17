/**
 * Pure popover geometry (S16/S17). DOM-free: the popover component reads
 * `getBoundingClientRect()` + viewport size and calls
 * `computePopoverPosition`; bun tests cover every rule here.
 *
 * Rules:
 * - below the anchor by `gap`, flipping above when the list would run
 *   past the viewport bottom and fits above instead;
 * - width is at least the anchor width, at most 520 (`maxWidth`);
 * - narrow screens shrink the width to the viewport minus margins;
 * - left/top are always clamped into the viewport (never clipped).
 */

export type AnchorRect = {
  top: number;
  left: number;
  bottom: number;
  width: number;
};

export type ViewportSize = {
  width: number;
  height: number;
};

export type PopoverPlacement = {
  top: number;
  left: number;
  width: number;
  /** Height budget used for the flip decision; apply as `max-height`. */
  maxHeight: number;
  placement: 'above' | 'below';
};

export type PopoverPositionOptions = {
  /** Gap between anchor and popover (default 6). */
  gap?: number;
  /** List height budget (default 360, cult `min(360px, 50vh)` in CSS). */
  maxHeight?: number;
  /** Max popover width (default 520). */
  maxWidth?: number;
  /** Minimum distance to any viewport edge (default 8). */
  margin?: number;
};

export const AUTOCOMPLETE_MAX_WIDTH = 520;
export const AUTOCOMPLETE_MAX_HEIGHT = 360;
export const AUTOCOMPLETE_GAP = 6;
export const AUTOCOMPLETE_VIEWPORT_MARGIN = 8;

export function computePopoverPosition(
  anchor: AnchorRect,
  viewport: ViewportSize,
  options: PopoverPositionOptions = {},
): PopoverPlacement {
  const gap = options.gap ?? AUTOCOMPLETE_GAP;
  const margin = options.margin ?? AUTOCOMPLETE_VIEWPORT_MARGIN;
  const maxWidth = options.maxWidth ?? AUTOCOMPLETE_MAX_WIDTH;
  // Mirror the CSS cap `min(360px, 50vh)` so the flip decision matches the
  // rendered height on short viewports.
  const maxHeight = Math.min(
    options.maxHeight ?? AUTOCOMPLETE_MAX_HEIGHT,
    Math.max(0, viewport.height * 0.5),
    Math.max(0, viewport.height - margin * 2),
  );

  // Width: at least the anchor, at most 520; narrow screens shrink to fit.
  let width = Math.min(Math.max(anchor.width, 0), maxWidth);
  const availableWidth = Math.max(0, viewport.width - margin * 2);
  if (width > availableWidth) width = availableWidth;

  const belowTop = anchor.bottom + gap;
  const aboveTop = anchor.top - gap - maxHeight;
  const fitsBelow = belowTop + maxHeight <= viewport.height - margin;
  const fitsAbove = aboveTop >= margin;
  const placement: 'above' | 'below' = !fitsBelow && fitsAbove ? 'above' : 'below';
  const top = placement === 'above'
    ? Math.max(margin, aboveTop)
    : Math.min(Math.max(margin, belowTop), Math.max(margin, viewport.height - margin - maxHeight));

  const maxLeft = Math.max(margin, viewport.width - width - margin);
  const left = Math.max(margin, Math.min(anchor.left, maxLeft));

  return { top, left, width, maxHeight, placement };
}
