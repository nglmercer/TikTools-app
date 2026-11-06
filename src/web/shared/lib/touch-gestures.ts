/**
 * Small touch-gesture helpers for the mobile layout. They use Touch Events
 * (not Pointer Events) because a pointer stream is cancelled as soon as the
 * browser starts a native scroll, which is exactly when a pull begins.
 * All listeners are passive: gestures never block native scrolling.
 */

export type SwipeDirection = 'left' | 'right' | 'up' | 'down';

interface Point { x: number; y: number }

const firstTouch = (event: TouchEvent): Point | null => {
  const touch = event.touches[0] ?? event.changedTouches[0];
  return touch ? { x: touch.clientX, y: touch.clientY } : null;
};

/** Text entry and range drags own their own touch movement. */
export function isGestureExempt(target: EventTarget | null): boolean {
  if (!(target instanceof Element)) return false;
  return target.closest('input, textarea, select, [contenteditable="true"], [data-gesture="off"]') !== null;
}

/** True when the target, or any scroller between it and `root`, is scrolled away from the top. */
export function scrolledAwayFromTop(target: EventTarget | null, root: Element): boolean {
  let node = target instanceof Element ? target : null;
  while (node && node !== root.parentElement) {
    if (node.scrollTop > 0) return true;
    node = node.parentElement;
  }
  return false;
}

export interface SwipeOptions {
  onSwipe: (direction: SwipeDirection) => void;
  /** Minimum travel along the main axis, in px. */
  threshold?: number;
  /** Only report swipes that start within `edge` px of the left edge. */
  edge?: number;
  /** Return false to ignore a gesture before it starts. */
  enabled?: () => boolean;
}

/** Reports one dominant-axis swipe per touch sequence. Returns a disposer. */
export function onSwipe(target: EventTarget, options: SwipeOptions): () => void {
  const threshold = options.threshold ?? 56;
  let start: Point | null = null;

  const begin = (event: Event): void => {
    const touch = event as TouchEvent;
    start = null;
    if (touch.touches.length !== 1 || options.enabled?.() === false || isGestureExempt(touch.target)) return;
    const point = firstTouch(touch);
    if (!point || (options.edge !== undefined && point.x > options.edge)) return;
    start = point;
  };
  const end = (event: Event): void => {
    const from = start;
    start = null;
    const to = firstTouch(event as TouchEvent);
    if (!from || !to) return;
    const dx = to.x - from.x;
    const dy = to.y - from.y;
    // Require a clearly dominant axis so diagonal scrolls never count.
    if (Math.abs(dx) >= threshold && Math.abs(dx) > Math.abs(dy) * 1.5) {
      options.onSwipe(dx > 0 ? 'right' : 'left');
    } else if (Math.abs(dy) >= threshold && Math.abs(dy) > Math.abs(dx) * 1.5) {
      options.onSwipe(dy > 0 ? 'down' : 'up');
    }
  };
  const cancel = (): void => { start = null; };

  target.addEventListener('touchstart', begin, { passive: true });
  target.addEventListener('touchend', end, { passive: true });
  target.addEventListener('touchcancel', cancel, { passive: true });
  return () => {
    target.removeEventListener('touchstart', begin);
    target.removeEventListener('touchend', end);
    target.removeEventListener('touchcancel', cancel);
  };
}

export interface PullOptions {
  /** Distance (after resistance) that arms the refresh. */
  threshold?: number;
  /** Visual pull distance, after resistance, in px. */
  onPull: (distance: number, armed: boolean) => void;
  onRefresh: () => void;
}

const PULL_RESISTANCE = 0.5;
const PULL_MAX = 96;

/**
 * Pull-to-refresh on `root`: starts only when every scroller under the
 * finger is at the top, and only for a mostly vertical downward drag.
 */
export function onPullToRefresh(root: HTMLElement, options: PullOptions): () => void {
  const threshold = options.threshold ?? 64;
  let start: Point | null = null;
  let pulling = false;
  let distance = 0;

  const reset = (): void => {
    start = null;
    pulling = false;
    if (distance !== 0) {
      distance = 0;
      options.onPull(0, false);
    }
  };
  const begin = (event: Event): void => {
    const touch = event as TouchEvent;
    reset();
    if (touch.touches.length !== 1 || isGestureExempt(touch.target) || scrolledAwayFromTop(touch.target, root)) return;
    start = firstTouch(touch);
  };
  const move = (event: Event): void => {
    const touch = event as TouchEvent;
    const point = firstTouch(touch);
    if (!start || !point || touch.touches.length !== 1) return;
    const dx = point.x - start.x;
    const dy = point.y - start.y;
    if (!pulling) {
      // Decide once: horizontal or upward motion abandons the gesture.
      if (Math.abs(dx) > 10 && Math.abs(dx) > Math.abs(dy)) { start = null; return; }
      if (dy < -4) { start = null; return; }
      if (dy < 10) return;
      pulling = true;
    }
    distance = Math.max(0, Math.min(PULL_MAX, (dy - 10) * PULL_RESISTANCE));
    options.onPull(distance, distance >= threshold);
  };
  const end = (): void => {
    const armed = pulling && distance >= threshold;
    reset();
    if (armed) options.onRefresh();
  };

  root.addEventListener('touchstart', begin, { passive: true });
  root.addEventListener('touchmove', move, { passive: true });
  root.addEventListener('touchend', end, { passive: true });
  root.addEventListener('touchcancel', reset, { passive: true });
  return () => {
    root.removeEventListener('touchstart', begin);
    root.removeEventListener('touchmove', move);
    root.removeEventListener('touchend', end);
    root.removeEventListener('touchcancel', reset);
  };
}

export interface LongPressOptions {
  onLongPress: (point: Point, target: EventTarget | null) => void;
  delay?: number;
  /** Movement that cancels the press, in px. */
  tolerance?: number;
}

/** Touch long-press, for context menus on platforms without `contextmenu` on touch (iOS). */
export function onLongPress(target: EventTarget, options: LongPressOptions): () => void {
  const delay = options.delay ?? 500;
  const tolerance = options.tolerance ?? 10;
  let timer: ReturnType<typeof setTimeout> | null = null;
  let start: Point | null = null;

  const clear = (): void => {
    if (timer !== null) clearTimeout(timer);
    timer = null;
    start = null;
  };
  const begin = (event: Event): void => {
    const touch = event as TouchEvent;
    clear();
    if (touch.touches.length !== 1 || isGestureExempt(touch.target)) return;
    start = firstTouch(touch);
    const point = start;
    const origin = touch.target;
    timer = setTimeout(() => {
      timer = null;
      if (point) options.onLongPress(point, origin);
    }, delay);
  };
  const move = (event: Event): void => {
    const point = firstTouch(event as TouchEvent);
    if (start && point && Math.hypot(point.x - start.x, point.y - start.y) > tolerance) clear();
  };

  target.addEventListener('touchstart', begin, { passive: true });
  target.addEventListener('touchmove', move, { passive: true });
  target.addEventListener('touchend', clear, { passive: true });
  target.addEventListener('touchcancel', clear, { passive: true });
  return () => {
    clear();
    target.removeEventListener('touchstart', begin);
    target.removeEventListener('touchmove', move);
    target.removeEventListener('touchend', clear);
    target.removeEventListener('touchcancel', clear);
  };
}
