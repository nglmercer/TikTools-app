export type TooltipPosition = 'top' | 'bottom' | 'left' | 'right';

export type TooltipRect = {
  top: number;
  left: number;
  width: number;
  height: number;
};

export type TooltipViewport = {
  width: number;
  height: number;
};

export type TooltipPlacement = {
  top: number;
  left: number;
  position: TooltipPosition;
};

const OPPOSITE: Record<TooltipPosition, TooltipPosition> = {
  top: 'bottom',
  bottom: 'top',
  left: 'right',
  right: 'left',
};

function candidate(
  trigger: TooltipRect,
  size: TooltipViewport,
  position: TooltipPosition,
  gap: number,
): TooltipPlacement {
  switch (position) {
    case 'top':
      return {
        top: trigger.top - size.height - gap,
        left: trigger.left + trigger.width / 2 - size.width / 2,
        position,
      };
    case 'bottom':
      return {
        top: trigger.top + trigger.height + gap,
        left: trigger.left + trigger.width / 2 - size.width / 2,
        position,
      };
    case 'left':
      return {
        top: trigger.top + trigger.height / 2 - size.height / 2,
        left: trigger.left - size.width - gap,
        position,
      };
    case 'right':
      return {
        top: trigger.top + trigger.height / 2 - size.height / 2,
        left: trigger.left + trigger.width + gap,
        position,
      };
  }
}

function fits(placement: TooltipPlacement, size: TooltipViewport, viewport: TooltipViewport, pad: number): boolean {
  return (
    placement.top >= pad
    && placement.left >= pad
    && placement.top + size.height <= viewport.height - pad
    && placement.left + size.width <= viewport.width - pad
  );
}

/**
 * Place a portal tooltip near its trigger. Tries the preferred side, flips to
 * the opposite side on overflow, then clamps into the viewport so the tip is
 * never painted off-screen.
 */
export function computeTooltipPosition(
  trigger: TooltipRect,
  size: TooltipViewport,
  preferred: TooltipPosition,
  viewport: TooltipViewport,
  gap = 8,
): TooltipPlacement {
  const pad = 8;
  const order: TooltipPosition[] = [preferred, OPPOSITE[preferred]];
  for (const position of (['top', 'bottom', 'left', 'right'] as const)) {
    if (!order.includes(position)) order.push(position);
  }
  for (const position of order) {
    const placement = candidate(trigger, size, position, gap);
    if (fits(placement, size, viewport, pad)) return placement;
  }
  const fallback = candidate(trigger, size, preferred, gap);
  return {
    top: Math.min(Math.max(fallback.top, pad), Math.max(pad, viewport.height - size.height - pad)),
    left: Math.min(Math.max(fallback.left, pad), Math.max(pad, viewport.width - size.width - pad)),
    position: preferred,
  };
}
