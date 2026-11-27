/**
 * Motion for code that animates with the Web Animations API (CSS handles the rest). Durations and the travel
 * distance come from the design tokens in `app/styles/tokens.css`, so one source decides them and
 * `prefers-reduced-motion` (tokens become 0) turns every animation off without a second media query here.
 */
export type MotionToken = '--duration-fast' | '--duration-normal' | '--duration-slow' | '--motion-shift';

const UNIT = /^(-?\d*\.?\d+)(ms|s|px)?$/;

/** A token in ms (or px for the shift) as a number; 0 when it is unset, unparsable or reduced to zero. */
export function motionValue(token: MotionToken, root: Element = document.documentElement): number {
  const raw = getComputedStyle(root).getPropertyValue(token).trim();
  const match = UNIT.exec(raw);
  if (!match) return 0;
  const value = Number(match[1]);
  return match[2] === 's' ? value * 1000 : value;
}

/** Fade (and optionally travel) `element` in. Does nothing when motion is reduced or the API is missing. */
export function enter(element: Element, { shift = 0, token = '--duration-normal' }: { shift?: number; token?: MotionToken } = {}): void {
  const duration = motionValue(token);
  if (duration === 0 || typeof element.animate !== 'function') return;
  const travel = shift === 0 ? 0 : shift * Math.min(1, motionValue('--motion-shift') / 8);
  element.animate(
    [{ opacity: 0, transform: `translateY(${travel}px)` }, { opacity: 1, transform: 'translateY(0)' }],
    { duration, easing: 'cubic-bezier(0.2, 0, 0, 1)' },
  );
}
