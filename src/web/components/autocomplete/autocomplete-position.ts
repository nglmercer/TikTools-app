/**
 * Measured popover geometry. DOM-free: the popover renders hidden, measures
 * the real panel, then calls `placeMeasuredPopover` with that size; bun
 * tests cover every rule here.
 *
 * Rules:
 * - placement uses the popup's ACTUAL desired size, never a theoretical
 *   height budget — a one-row hint by a bottom input stays below instead
 *   of flipping on a 360px guess;
 * - below the anchor by `gap`, flipping above only when the real height
 *   fits above but not below;
 * - the popup never intersects its anchor: a below popup starts at
 *   `anchor.bottom + gap` and is never clamped upward through the input;
 *   an above popup ends at `anchor.top - gap`;
 * - when neither side fits, the roomier side wins and `maxHeight` is set
 *   to exactly the space available there;
 * - width clamps into the viewport horizontally; the height budget
 *   mirrors `min(maxHeight, 50vh)` so tall lists scroll instead of
 *   overflowing short viewports.
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

/** The popup's real desired size, measured from the rendered panel. */
export type PopupSize = {
  width: number;
  height: number;
};

export type MeasuredPopoverPlacement = {
  top: number;
  left: number;
  width: number;
  /** Explicit height budget to apply as `max-height`. */
  maxHeight: number;
  placement: 'above' | 'below';
};

export type MeasuredPopoverOptions = {
  /** Gap between anchor and popover (default 6). */
  gap?: number;
  /** Minimum distance to any viewport edge (default 8). */
  margin?: number;
  /** Minimum popup width (default 0; narrow screens may shrink below it). */
  minWidth?: number;
  /** Max popup width (default 520). */
  maxWidth?: number;
  /** Height budget cap (default 360, still capped by `50vh`). */
  maxHeight?: number;
};

export const AUTOCOMPLETE_MAX_WIDTH = 520;
export const AUTOCOMPLETE_MAX_HEIGHT = 360;
export const AUTOCOMPLETE_GAP = 6;
export const AUTOCOMPLETE_VIEWPORT_MARGIN = 8;
/** Default desired width when the anchor is a caret, not a field box. */
export const AUTOCOMPLETE_PREFERRED_WIDTH = 360;
/** Compact preset hint: only the width it needs, never the full input. */
export const AUTOCOMPLETE_PRESET_MIN_WIDTH = 320;
export const AUTOCOMPLETE_PRESET_MAX_WIDTH = 420;

/** Clamp a desired popup width into the viewport and the width options. */
export function resolvePopupWidth(
  desired: number,
  viewport: ViewportSize,
  options: Pick<MeasuredPopoverOptions, 'minWidth' | 'maxWidth' | 'margin'> = {},
): number {
  const margin = options.margin ?? AUTOCOMPLETE_VIEWPORT_MARGIN;
  const maxWidth = options.maxWidth ?? AUTOCOMPLETE_MAX_WIDTH;
  const availableWidth = Math.max(0, viewport.width - margin * 2);
  const floored = Math.max(desired, options.minWidth ?? 0);
  return Math.max(0, Math.min(floored, maxWidth, availableWidth));
}

/**
 * Place a measured popup next to its anchor. `popup` is the real panel
 * size (width at the resolved width, height from `scrollHeight`), so the
 * flip decision matches what the user will actually see.
 */
export function placeMeasuredPopover(
  anchor: AnchorRect,
  popup: PopupSize,
  viewport: ViewportSize,
  options: MeasuredPopoverOptions = {},
): MeasuredPopoverPlacement {
  const gap = options.gap ?? AUTOCOMPLETE_GAP;
  const margin = options.margin ?? AUTOCOMPLETE_VIEWPORT_MARGIN;
  // Mirror the CSS cap `min(maxHeight, 50vh)` so tall lists scroll instead
  // of overflowing short viewports.
  const heightCap = Math.min(
    options.maxHeight ?? AUTOCOMPLETE_MAX_HEIGHT,
    Math.max(0, viewport.height * 0.5),
    Math.max(0, viewport.height - margin * 2),
  );
  const height = Math.max(0, Math.min(popup.height, heightCap));
  const width = resolvePopupWidth(popup.width, viewport, options);

  const belowTop = anchor.bottom + gap;
  const spaceBelow = viewport.height - margin - belowTop;
  const spaceAbove = anchor.top - gap - margin;
  const fitsBelow = height <= spaceBelow;
  const fitsAbove = height <= spaceAbove;
  let placement: 'above' | 'below';
  if (fitsBelow) placement = 'below';
  else if (fitsAbove) placement = 'above';
  else placement = spaceBelow >= spaceAbove ? 'below' : 'above';

  const available = placement === 'below' ? spaceBelow : spaceAbove;
  const maxHeight = Math.max(0, Math.min(height, available));
  // Below never clamps upward through the anchor, even when constrained:
  // it starts at the anchor edge and scrolls. Above ends at the anchor
  // edge by construction (`top + maxHeight === anchor.top - gap`).
  const top = placement === 'below' ? belowTop : anchor.top - gap - maxHeight;

  const maxLeft = Math.max(margin, viewport.width - width - margin);
  const left = Math.max(margin, Math.min(anchor.left, maxLeft));

  return { top, left, width, maxHeight, placement };
}

/**
 * First-open rule: the popup stays hidden until a real measurement has
 * produced a placement, so it never flashes at a guessed position.
 */
export function resolvePopoverVisibility(input: {
  open: boolean;
  measured: boolean;
  placement: MeasuredPopoverPlacement | null;
}): 'hidden' | 'visible' {
  return input.open && input.measured && input.placement ? 'visible' : 'hidden';
}
