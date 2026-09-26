import {
  placeMeasuredPopover,
  resolvePopupWidth,
  type AnchorRect,
  type MeasuredPopoverOptions,
  type MeasuredPopoverPlacement,
  type PopupSize,
  type ViewportSize,
} from './autocomplete-position.ts';
import { resolveDesiredPopupWidth, type AutocompleteAnchorMode } from './autocomplete-anchors.ts';

/**
 * Measured popover lifecycle, DOM-free. The Vue shell owns the real
 * elements and listeners; this engine owns the rules:
 *
 * - closed or unmeasured means hidden — there is no guessed position,
 *   so first-open can never flash at `{top: 0, left: 0}`;
 * - every layout signal (resize, scroll, ResizeObserver, cursor move,
 *   option change) collapses through one frame scheduler into a single
 *   measure-then-place pass;
 * - placement always uses the freshly measured panel size, so the flip
 *   decision matches the popup the user will actually see.
 */

export type PopoverEngineEnvironment = {
  /** Current anchor rectangle in viewport coordinates (null = not ready). */
  readAnchor: () => AnchorRect | null;
  /** Current viewport size (null = not ready). */
  readViewport: () => ViewportSize | null;
  /**
   * Measure the panel at `provisionalWidth` and return its natural size
   * (null = not laid out yet). The shell renders the panel hidden and
   * unclamped for this read.
   */
  measurePopup: (provisionalWidth: number) => PopupSize | null;
  requestFrame: (callback: () => void) => number;
  cancelFrame: (handle: number) => void;
};

export type PopoverEngineOptions = MeasuredPopoverOptions & {
  /** Desired width before clamping (caret/compact anchors). */
  preferredWidth?: number;
  /** Anchor kind: caret anchors size from the preferred width, field anchors match their control. */
  anchorMode?: AutocompleteAnchorMode;
};

export type PopoverEngineSnapshot = {
  open: boolean;
  /** True once a real measurement has produced a placement. */
  measured: boolean;
  placement: MeasuredPopoverPlacement | null;
  /** Width the hidden panel should render at for its next measurement. */
  provisionalWidth: number | null;
};

export function createMeasuredPopoverEngine(
  env: PopoverEngineEnvironment,
  options: PopoverEngineOptions = {},
): {
  snapshot: () => PopoverEngineSnapshot;
  subscribe: (listener: () => void) => () => void;
  setOpen: (open: boolean) => void;
  setOptions: (next: PopoverEngineOptions) => void;
  /** One entry point for every layout signal; collapses into one frame. */
  handleLayoutChange: () => void;
  /** Synchronous pass (tests, or an already-laid-out frame). */
  computeNow: () => boolean;
  dispose: () => void;
} {
  let currentOptions = options;
  let open = false;
  let measured = false;
  let placement: MeasuredPopoverPlacement | null = null;
  let provisionalWidth: number | null = null;
  let frame: number | null = null;
  let disposed = false;
  const listeners = new Set<() => void>();

  const snapshot = (): PopoverEngineSnapshot => ({ open, measured, placement, provisionalWidth });

  const emit = (): void => {
    for (const listener of listeners) listener();
  };

  const samePlacement = (left: MeasuredPopoverPlacement | null, right: MeasuredPopoverPlacement | null): boolean => {
    if (left === right) return true;
    if (!left || !right) return false;
    return (
      left.top === right.top
      && left.left === right.left
      && left.width === right.width
      && left.maxHeight === right.maxHeight
      && left.placement === right.placement
    );
  };

  const cancelPendingFrame = (): void => {
    if (frame !== null) {
      env.cancelFrame(frame);
      frame = null;
    }
  };

  const computeNow = (): boolean => {
    if (!open || disposed) return false;
    const anchor = env.readAnchor();
    const viewport = env.readViewport();
    if (!anchor || !viewport) return false;
    provisionalWidth = resolvePopupWidth(
      resolveDesiredPopupWidth({
        anchorWidth: anchor.width,
        mode: currentOptions.anchorMode ?? 'field',
        preferredWidth: currentOptions.preferredWidth,
      }),
      viewport,
      currentOptions,
    );
    const size = env.measurePopup(provisionalWidth);
    if (!size) return false;
    const next = placeMeasuredPopover(anchor, { width: provisionalWidth, height: size.height }, viewport, currentOptions);
    const changed = !measured || !samePlacement(placement, next);
    placement = next;
    measured = true;
    if (changed) emit();
    return true;
  };

  const handleLayoutChange = (): void => {
    if (!open || disposed) return;
    if (frame !== null) return;
    frame = env.requestFrame(() => {
      frame = null;
      computeNow();
    });
  };

  return {
    snapshot,
    subscribe: (listener) => {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
    setOpen: (next) => {
      if (disposed) return;
      if (next === open) return;
      open = next;
      cancelPendingFrame();
      measured = false;
      placement = null;
      provisionalWidth = null;
      emit();
      if (open) handleLayoutChange();
    },
    setOptions: (next) => {
      currentOptions = next;
      handleLayoutChange();
    },
    handleLayoutChange,
    computeNow,
    dispose: () => {
      disposed = true;
      cancelPendingFrame();
      listeners.clear();
    },
  };
}
