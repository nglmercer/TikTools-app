<script lang="tsx">
import { nextTick, onMounted, onUnmounted, ref, Teleport, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import {
  AUTOCOMPLETE_GAP,
  AUTOCOMPLETE_MAX_HEIGHT,
  AUTOCOMPLETE_MAX_WIDTH,
  AUTOCOMPLETE_PREFERRED_WIDTH,
  AUTOCOMPLETE_VIEWPORT_MARGIN,
  resolvePopoverVisibility,
  resolvePopupWidth,
  type AnchorRect,
  type ViewportSize,
} from './autocomplete-position.ts';
import {
  getCaretAnchorRect,
  resolveAnchorRect,
  resolveDesiredPopupWidth,
  type AutocompleteAnchorMode,
} from './autocomplete-anchors.ts';
import {
  createMeasuredPopoverEngine,
  type PopoverEngineOptions,
} from './autocomplete-popover-engine.ts';

export type AutocompletePopoverProps = {
  /** Anchor element (usually the field box). Null renders nothing. */
  anchor: HTMLElement | null;
  open: boolean;
  /** Extra key (e.g. caret offset) that re-runs positioning. */
  updateKey?: number | string;
  gap?: number;
  maxHeight?: number;
  maxWidth?: number;
  margin?: number;
  /** `field` anchors to the control box; `caret` to a virtual caret rect. */
  anchorMode?: AutocompleteAnchorMode;
  /** Text control for caret mode (mirror source). */
  input?: HTMLInputElement | HTMLTextAreaElement | null;
  /** Caret offset for caret mode (defaults to the live selection). */
  caretOffset?: number;
  /** Explicit anchor rectangle (value or provider); overrides measurement. */
  anchorRect?: AnchorRect | (() => AnchorRect | null) | null;
  /** Desired width before clamping (compact hints, caret anchors). */
  preferredWidth?: number;
  minWidth?: number;
  /** Pointer presence over the panel (interaction lifetime). */
  onPopupPointerChange?: (inside: boolean) => void;
};

/**
 * Document-level popup shell: Teleports to `body`, `position: fixed`
 * at `var(--z-autocomplete)`, so modal/canvas overflow never clips the
 * list. Renders hidden until a real panel measurement produces a
 * placement, flips on the measured height (never a budget guess), and
 * repositions through one frame scheduler on resize, scroll,
 * visualViewport, ResizeObserver (anchor/panel/document), cursor moves,
 * and option changes. Autosave rerenders only reposition — they never
 * close the popup (lifetime lives in `use-autocomplete.ts`).
 */
export const AutocompletePopover = defineVueComponent<AutocompletePopoverProps>(
  ['anchor', 'open', 'updateKey', 'gap', 'maxHeight', 'maxWidth', 'margin', 'anchorMode', 'input', 'caretOffset', 'anchorRect', 'preferredWidth', 'minWidth', 'onPopupPointerChange'],
  (props, context) => {
    const popoverRef = ref<HTMLElement | null>(null);

    const readAnchorElement = (): AnchorRect | null => {
      const anchor = props.anchor;
      if (!anchor || !anchor.isConnected) return null;
      const rect = anchor.getBoundingClientRect();
      return { top: rect.top, left: rect.left, bottom: rect.bottom, width: rect.width };
    };

    const readCaretOffset = (): number => {
      if (typeof props.caretOffset === 'number') return props.caretOffset;
      const control = props.input;
      if (!control) return 0;
      try {
        return control.selectionStart ?? control.value.length;
      } catch {
        return control.value.length;
      }
    };

    const readAnchor = (): AnchorRect | null => {
      const override = props.anchorRect;
      if (typeof override === 'function') {
        try {
          const rect = override();
          if (rect) return rect;
        } catch {
          // Fall through to measured anchors.
        }
      } else if (override) {
        return override;
      }
      const field = readAnchorElement();
      if ((props.anchorMode ?? 'field') === 'caret') {
        const caret = getCaretAnchorRect(props.input ?? null, readCaretOffset());
        return resolveAnchorRect({ mode: 'caret', field, caret });
      }
      return field;
    };

    const readViewport = (): ViewportSize | null => {
      if (typeof window === 'undefined') return null;
      return { width: window.innerWidth, height: window.innerHeight };
    };

    const engineOptions = (): PopoverEngineOptions => ({
      gap: props.gap ?? AUTOCOMPLETE_GAP,
      maxHeight: props.maxHeight ?? AUTOCOMPLETE_MAX_HEIGHT,
      maxWidth: props.maxWidth ?? AUTOCOMPLETE_MAX_WIDTH,
      margin: props.margin ?? AUTOCOMPLETE_VIEWPORT_MARGIN,
      preferredWidth: props.preferredWidth,
      minWidth: props.minWidth,
      anchorMode: props.anchorMode ?? 'field',
    });

    const engine = createMeasuredPopoverEngine(
      {
        readAnchor,
        readViewport,
        measurePopup: () => {
          const panel = popoverRef.value;
          if (!panel || !panel.isConnected) return null;
          // `scrollHeight` is the full content height even while a previous
          // `max-height` clamps the rendered box.
          const height = panel.scrollHeight > 0 ? panel.scrollHeight : panel.offsetHeight;
          if (!(height > 0)) return null;
          return { width: 0, height };
        },
        requestFrame: (callback) => {
          if (typeof requestAnimationFrame === 'function') return requestAnimationFrame(callback);
          return setTimeout(callback, 0) as unknown as number;
        },
        cancelFrame: (handle) => {
          if (typeof cancelAnimationFrame === 'function') cancelAnimationFrame(handle);
          else clearTimeout(handle);
        },
      },
      engineOptions(),
    );

    const engineState = ref(engine.snapshot());
    const stopEngine = engine.subscribe(() => {
      engineState.value = engine.snapshot();
    });

    const onLayoutSignal = (): void => {
      engine.handleLayoutChange();
    };

    let resizeObserver: ResizeObserver | null = null;
    let observedAnchor: HTMLElement | null = null;
    let observedPanel: HTMLElement | null = null;
    let observedBody = false;

    const observe = (): void => {
      if (typeof ResizeObserver === 'undefined' || typeof document === 'undefined') return;
      if (!resizeObserver) resizeObserver = new ResizeObserver(onLayoutSignal);
      const anchor = props.anchor && props.anchor.isConnected ? props.anchor : null;
      if (observedAnchor && observedAnchor !== anchor) resizeObserver.unobserve(observedAnchor);
      if (anchor && anchor !== observedAnchor) resizeObserver.observe(anchor);
      observedAnchor = anchor;
      const panel = props.open ? popoverRef.value : null;
      if (observedPanel && observedPanel !== panel) resizeObserver.unobserve(observedPanel);
      if (panel && panel !== observedPanel) resizeObserver.observe(panel);
      observedPanel = panel;
      // Translations (autosave banners, validation messages, card growth)
      // move the anchor without resizing it; the body still changes size.
      if (document.body && !observedBody) {
        resizeObserver.observe(document.body);
        observedBody = true;
      }
    };

    const unobservePanel = (): void => {
      if (resizeObserver && observedPanel) resizeObserver.unobserve(observedPanel);
      observedPanel = null;
    };

    const visualViewport = (): VisualViewport | null =>
      typeof window === 'undefined' ? null : window.visualViewport ?? null;

    onMounted(() => {
      engine.setOpen(props.open);
      if (typeof window !== 'undefined') {
        window.addEventListener('resize', onLayoutSignal);
        window.addEventListener('scroll', onLayoutSignal, true);
        visualViewport()?.addEventListener('resize', onLayoutSignal);
        visualViewport()?.addEventListener('scroll', onLayoutSignal);
      }
      observe();
    });
    onUnmounted(() => {
      if (typeof window !== 'undefined') {
        window.removeEventListener('resize', onLayoutSignal);
        window.removeEventListener('scroll', onLayoutSignal, true);
        visualViewport()?.removeEventListener('resize', onLayoutSignal);
        visualViewport()?.removeEventListener('scroll', onLayoutSignal);
      }
      resizeObserver?.disconnect();
      resizeObserver = null;
      observedAnchor = null;
      observedPanel = null;
      observedBody = false;
      stopEngine();
      engine.dispose();
    });

    watch(() => props.open, (open) => {
      if (!open) unobservePanel();
      engine.setOpen(open);
      void nextTick(() => {
        observe();
        engine.handleLayoutChange();
      });
    });
    watch(
      () => [
        props.anchor,
        props.input,
        props.caretOffset,
        props.updateKey,
        props.anchorMode,
        props.anchorRect,
        props.preferredWidth,
        props.minWidth,
        props.gap,
        props.maxHeight,
        props.maxWidth,
        props.margin,
      ],
      () => {
        engine.setOptions(engineOptions());
        engine.handleLayoutChange();
        observe();
      },
      { flush: 'post' },
    );
    watch(popoverRef, () => {
      observe();
      engine.handleLayoutChange();
    });

    const hiddenWidth = (): number => {
      const anchor = readAnchor();
      const viewport = readViewport();
      const desired = resolveDesiredPopupWidth({
        anchorWidth: anchor?.width ?? AUTOCOMPLETE_PREFERRED_WIDTH,
        mode: props.anchorMode ?? 'field',
        preferredWidth: props.preferredWidth,
      });
      if (!anchor || !viewport) return desired;
      return resolvePopupWidth(desired, viewport, engineOptions());
    };

    return () => {
      if (!props.open || typeof document === 'undefined' || !document.body) return null;
      const hasAnchorSource = Boolean(props.anchorRect)
        || Boolean(props.anchor)
        || ((props.anchorMode ?? 'field') === 'caret' && Boolean(props.input));
      if (!hasAnchorSource) return null;
      const state = engineState.value;
      const visibility = resolvePopoverVisibility({ open: props.open, measured: state.measured, placement: state.placement });
      const style: Record<string, string> = {};
      if (state.placement && visibility === 'visible') {
        style.top = `${state.placement.top}px`;
        style.left = `${state.placement.left}px`;
        style.width = `${state.placement.width}px`;
        style.maxHeight = `${state.placement.maxHeight}px`;
      } else {
        // Hidden first paint: correct width (so the measured height wraps
        // the same way), no position guess, no pointer capture.
        style.width = `${hiddenWidth()}px`;
        style.visibility = 'hidden';
        style.pointerEvents = 'none';
      }
      return (
        <Teleport to="body">
          <div
            ref={popoverRef}
            class="autocomplete-popover"
            data-placement={state.placement?.placement ?? 'below'}
            data-measured={state.measured ? 'true' : 'false'}
            data-anchor-mode={props.anchorMode ?? 'field'}
            style={style}
            onPointerenter={() => props.onPopupPointerChange?.(true)}
            onPointerleave={() => props.onPopupPointerChange?.(false)}
          >
            {context.slots.default?.()}
          </div>
        </Teleport>
      );
    };
  },
);

export default AutocompletePopover;
</script>
