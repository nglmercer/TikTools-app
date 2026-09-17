<script lang="tsx">
import { onMounted, onUnmounted, ref, Teleport, watch } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import {
  AUTOCOMPLETE_GAP,
  AUTOCOMPLETE_MAX_HEIGHT,
  AUTOCOMPLETE_MAX_WIDTH,
  AUTOCOMPLETE_VIEWPORT_MARGIN,
  computePopoverPosition,
  type PopoverPlacement,
} from './autocomplete-position.ts';

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
};

/**
 * Document-level popup shell (S16): Teleports to `body`, `position: fixed`
 * at `var(--z-autocomplete)`, so modal/canvas overflow never clips the
 * list. Flips above the anchor when short on room, clamps into the
 * viewport, and repositions on resize/scroll.
 */
export const AutocompletePopover = defineVueComponent<AutocompletePopoverProps>(
  ['anchor', 'open', 'updateKey', 'gap', 'maxHeight', 'maxWidth', 'margin'],
  (props, context) => {
    const position = ref<PopoverPlacement>({ top: 0, left: 0, width: 320, maxHeight: AUTOCOMPLETE_MAX_HEIGHT, placement: 'below' });

    const update = (): void => {
      if (!props.open || typeof window === 'undefined') return;
      const anchor = props.anchor;
      if (!anchor || !anchor.isConnected) return;
      const rect = anchor.getBoundingClientRect();
      position.value = computePopoverPosition(
        { top: rect.top, left: rect.left, bottom: rect.bottom, width: rect.width },
        { width: window.innerWidth, height: window.innerHeight },
        {
          gap: props.gap ?? AUTOCOMPLETE_GAP,
          maxHeight: props.maxHeight ?? AUTOCOMPLETE_MAX_HEIGHT,
          maxWidth: props.maxWidth ?? AUTOCOMPLETE_MAX_WIDTH,
          margin: props.margin ?? AUTOCOMPLETE_VIEWPORT_MARGIN,
        },
      );
    };

    onMounted(() => {
      update();
      if (typeof window !== 'undefined') {
        window.addEventListener('resize', update);
        window.addEventListener('scroll', update, true);
      }
    });
    onUnmounted(() => {
      if (typeof window !== 'undefined') {
        window.removeEventListener('resize', update);
        window.removeEventListener('scroll', update, true);
      }
    });
    watch(() => [props.open, props.anchor, props.updateKey], update, { flush: 'post' });

    return () => {
      if (!props.open || typeof document === 'undefined' || !document.body) return null;
      if (!props.anchor || !props.anchor.isConnected) return null;
      const style = {
        top: `${position.value.top}px`,
        left: `${position.value.left}px`,
        width: `${position.value.width}px`,
      };
      return (
        <Teleport to="body">
          <div class="autocomplete-popover" data-placement={position.value.placement} style={style}>
            {context.slots.default?.()}
          </div>
        </Teleport>
      );
    };
  },
);

export default AutocompletePopover;
</script>
