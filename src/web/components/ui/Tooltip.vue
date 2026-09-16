<script lang="tsx">
import { nextTick, onUnmounted, ref, Teleport } from 'vue';
import type { VNodeChild } from 'vue';
import { defineVueComponent } from '../../vue/component.ts';
import { computeTooltipPosition, type TooltipPlacement, type TooltipPosition } from './tooltip-logic.ts';

type TooltipProps = {
  /** Tooltip text. Empty text renders only the trigger. */
  text: string;
  position?: TooltipPosition;
  /** Wider bubble for longer explanations (matches the CSS wide variant). */
  wide?: boolean;
  disabled?: boolean;
  children: VNodeChild;
};

let tooltipSequence = 0;

/**
 * Portal tooltip: the bubble renders through Teleport into `body` with fixed
 * positioning, so it escapes local stacking contexts (backdrop-filter panes,
 * cards, scroll containers) that trap pseudo-element tooltips. Triggers must
 * stay self-labeled (`aria-label`); the portal duplicates the visual label.
 */
export const Tooltip = defineVueComponent<TooltipProps>(
  ['text', 'position', 'wide', 'disabled', 'children'],
  (props) => {
  const triggerRef = ref<HTMLElement | null>(null);
  const tipRef = ref<HTMLDivElement | null>(null);
  const open = ref(false);
  const placement = ref<TooltipPlacement | null>(null);
  const tipId = `ui-tooltip-${(tooltipSequence += 1)}`;

  const updatePlacement = (): void => {
    const trigger = triggerRef.value;
    const tip = tipRef.value;
    if (!trigger || !tip) return;
    const box = trigger.getBoundingClientRect();
    const size = tip.getBoundingClientRect();
    placement.value = computeTooltipPosition(
      { top: box.top, left: box.left, width: box.width, height: box.height },
      { width: size.width, height: size.height },
      props.position ?? 'top',
      { width: window.innerWidth, height: window.innerHeight },
    );
  };

  const show = (): void => {
    if (props.disabled || !props.text) return;
    if (open.value) {
      updatePlacement();
      return;
    }
    open.value = true;
    placement.value = null;
    void nextTick(() => updatePlacement());
  };

  const hide = (): void => {
    open.value = false;
    placement.value = null;
  };

  const onKeyDown = (event: KeyboardEvent): void => {
    if (event.key === 'Escape') hide();
  };

  const onViewportChange = (): void => {
    if (open.value) updatePlacement();
  };

  window.addEventListener('scroll', onViewportChange, true);
  window.addEventListener('resize', onViewportChange);
  onUnmounted(() => {
    window.removeEventListener('scroll', onViewportChange, true);
    window.removeEventListener('resize', onViewportChange);
  });

  return () => {
    const { text, wide, children } = props;
    const at = placement.value;
    return (
      <span
        ref={triggerRef}
        class="ui-tooltip-trigger"
        onMouseenter={show}
        onMouseleave={hide}
        onFocusin={show}
        onFocusout={hide}
        onKeydown={onKeyDown}
      >
        {children}
        {open.value && text ? (
          <Teleport to="body">
            <div
              ref={tipRef}
              id={tipId}
              role="tooltip"
              class={`ui-tooltip-portal${wide ? ' is-wide' : ''}`}
              data-position={at?.position ?? props.position ?? 'top'}
              style={{
                top: `${at?.top ?? -9999}px`,
                left: `${at?.left ?? -9999}px`,
                visibility: at ? 'visible' : 'hidden',
              }}
            >
              {text}
            </div>
          </Teleport>
        ) : null}
      </span>
    );
  };
  },
);

export default Tooltip;
</script>
