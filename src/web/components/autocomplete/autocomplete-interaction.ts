import type {
  AutocompleteController,
  AutocompleteControllerSnapshot,
  InsertResult,
} from './autocomplete-controller.ts';
import type { SuggestionRow } from './types.ts';

/**
 * Interaction lifetime for one autocomplete input. The filtering
 * controller stays pure (value/caret/focused in, snapshot out); this
 * session owns the messy human part: pointer-vs-focus races, deferred
 * blur, and rerender safety.
 *
 * Rules:
 * - a settings echo, `saveState` change, or controlled-value confirmation
 *   never dismisses the popup by itself — only state pushes do, and a
 *   rerender pushes nothing (`rerender()` proves it);
 * - blur defers one animation frame and closes only when focus AND the
 *   pointer are both outside the input/popover pair, so a row click can
 *   never lose to a blur/autosave race;
 * - explicit commit, Escape/dismiss, or zero results after editing close
 *   immediately; pointer flags reset on every commit.
 */

export type AutocompleteInteractionSnapshot = {
  /** Last known native focus state of the text input. */
  inputFocused: boolean;
  /** A row pointer press is in flight (down, not yet up/committed). */
  pointerSelecting: boolean;
  /** The pointer is currently over the popup panel. */
  popupPointerInside: boolean;
  /** A blur is waiting out its one-frame grace period. */
  blurDeferred: boolean;
};

export type AutocompleteFrameScheduler = {
  requestFrame: (callback: () => void) => number;
  cancelFrame: (handle: number) => void;
};

function defaultScheduler(): AutocompleteFrameScheduler {
  return {
    requestFrame: (callback) =>
      typeof requestAnimationFrame === 'function'
        ? requestAnimationFrame(callback)
        : (setTimeout(callback, 0) as unknown as number),
    cancelFrame: (handle) => {
      if (typeof cancelAnimationFrame === 'function') cancelAnimationFrame(handle);
      else clearTimeout(handle);
    },
  };
}

export type AutocompleteInteraction = AutocompleteController & {
  /** Current interaction flags (focus/pointer/blur-deferral). */
  interactionSnapshot: () => AutocompleteInteractionSnapshot;
  /** A row pointer press started (pointerdown/mousedown on a row). */
  beginPointerSelection: () => void;
  /** A row pointer press ended without committing (drag-off, cancel). */
  endPointerSelection: () => void;
  /** Track whether the pointer is over the popup panel. */
  setPopupPointerInside: (inside: boolean) => AutocompleteControllerSnapshot;
  /** Explicit rerender notification: must never change the snapshot. */
  rerender: () => AutocompleteControllerSnapshot;
  /** A commit landed: reset pointer/blur bookkeeping, keep focus state. */
  notifyCommitted: () => void;
};

export function createAutocompleteInteraction(input: {
  controller: AutocompleteController;
  schedule?: AutocompleteFrameScheduler;
  /** Fires when a deferred blur settles (the Vue glue refreshes its ref). */
  onSettled?: (snapshot: AutocompleteControllerSnapshot) => void;
}): AutocompleteInteraction {
  const controller = input.controller;
  const schedule = input.schedule ?? defaultScheduler();
  let value = '';
  let caret = 0;
  let domFocused = false;
  let pointerSelecting = false;
  let popupPointerInside = false;
  let pendingBlur = false;
  let frame: number | null = null;

  const cancelPendingFrame = (): void => {
    if (frame !== null) {
      schedule.cancelFrame(frame);
      frame = null;
    }
  };

  const settled = (snapshot: AutocompleteControllerSnapshot): AutocompleteControllerSnapshot => {
    input.onSettled?.(snapshot);
    return snapshot;
  };

  const closeFromBlur = (): AutocompleteControllerSnapshot => {
    pendingBlur = false;
    return settled(controller.update({ value, caret, focused: false }));
  };

  const settleBlur = (): void => {
    frame = null;
    if (!pendingBlur) return;
    // Refocused during the grace period: the pending blur is void.
    if (domFocused) {
      pendingBlur = false;
      return;
    }
    // The pointer is still inside the pair: stay open and keep waiting.
    // `endPointerSelection` / `setPopupPointerInside(false)` settle next.
    if (pointerSelecting || popupPointerInside) return;
    closeFromBlur();
  };

  const scheduleBlur = (): void => {
    pendingBlur = true;
    cancelPendingFrame();
    frame = schedule.requestFrame(settleBlur);
  };

  const maybeCloseAfterPointer = (): AutocompleteControllerSnapshot => {
    if (pendingBlur && !domFocused && !pointerSelecting && !popupPointerInside) {
      cancelPendingFrame();
      return closeFromBlur();
    }
    return controller.snapshot();
  };

  const interaction: AutocompleteInteraction = {
    snapshot: () => controller.snapshot(),
    update: (next) => {
      value = next.value;
      caret = next.caret;
      if (next.focused) {
        domFocused = true;
        pendingBlur = false;
        // A fresh input interaction supersedes any press the rows never
        // reported as ended (press, drag off, release outside).
        pointerSelecting = false;
        cancelPendingFrame();
        return controller.update({ value, caret, focused: true });
      }
      domFocused = false;
      scheduleBlur();
      // Stay open through the grace period; `settleBlur` closes.
      return controller.snapshot();
    },
    invoke: () => controller.invoke(),
    dismiss: () => {
      pendingBlur = false;
      pointerSelecting = false;
      popupPointerInside = false;
      cancelPendingFrame();
      return controller.dismiss();
    },
    hover: (globalIndex) => controller.hover(globalIndex),
    key: (keyValue) => {
      const result = controller.key(keyValue);
      if (result === 'dismissed') {
        pendingBlur = false;
        pointerSelecting = false;
        popupPointerInside = false;
        cancelPendingFrame();
      }
      return result;
    },
    pendingRow: () => controller.pendingRow(),
    commit: (commitValue, commitCaret) => {
      const result: (InsertResult & { row: SuggestionRow }) | null = controller.commit(
        commitValue,
        commitCaret,
      );
      if (result) {
        value = result.value;
        caret = result.caret;
        interaction.notifyCommitted();
      }
      return result;
    },
    interactionSnapshot: () => ({
      inputFocused: domFocused,
      pointerSelecting,
      popupPointerInside,
      blurDeferred: pendingBlur,
    }),
    beginPointerSelection: () => {
      pointerSelecting = true;
    },
    endPointerSelection: () => {
      pointerSelecting = false;
      maybeCloseAfterPointer();
    },
    setPopupPointerInside: (inside) => {
      popupPointerInside = inside;
      // Leaving the panel ends any press by definition: a later release
      // lands outside the rows, so no row pointerup will arrive.
      if (!inside) pointerSelecting = false;
      return maybeCloseAfterPointer();
    },
    rerender: () => controller.snapshot(),
    notifyCommitted: () => {
      pendingBlur = false;
      pointerSelecting = false;
      popupPointerInside = false;
      cancelPendingFrame();
    },
  };
  return interaction;
}
