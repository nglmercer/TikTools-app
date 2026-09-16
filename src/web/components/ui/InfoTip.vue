<script lang="tsx">
import { IconInfo } from '../icons/index.ts';
import { Tooltip } from './Tooltip.vue';
import type { TooltipPosition } from './tooltip-logic.ts';

type InfoTipProps = {
  /** Shown on hover and focus, and read by screen readers. */
  text: string;
  position?: TooltipPosition;
};

/**
 * The escape valve for a minimal form: the explanation lives behind an icon
 * instead of a paragraph under every field. The bubble is a portal tooltip so
 * it paints above cards, ribbons, and other stacking contexts.
 */
export function InfoTip({ text, position = 'top' }: InfoTipProps) {
  return (
    <Tooltip text={text} position={position} wide>
      <span
        class="ui-infotip"
        tabindex={0}
        role="note"
        aria-label={text}
      >
        <IconInfo size={13} />
      </span>
    </Tooltip>
  );
}

export default InfoTip;
</script>
