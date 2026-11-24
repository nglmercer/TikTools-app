import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { onLongPress, onPullToRefresh, onSwipe } from './touch-gestures.ts';

function touch(target: EventTarget, type: string, x: number, y: number, fingers = 1): void {
  const event = new Event(type, { bubbles: true, cancelable: true });
  const list = type === 'touchend' || type === 'touchcancel' ? [] : Array.from({ length: fingers }, () => ({ clientX: x, clientY: y }));
  Object.defineProperty(event, 'touches', { value: list });
  Object.defineProperty(event, 'changedTouches', { value: [{ clientX: x, clientY: y }] });
  target.dispatchEvent(event);
}

function drag(target: EventTarget, from: [number, number], to: [number, number]): void {
  touch(target, 'touchstart', ...from);
  const steps = 4;
  for (let step = 1; step <= steps; step += 1) {
    touch(target, 'touchmove', from[0] + ((to[0] - from[0]) * step) / steps, from[1] + ((to[1] - from[1]) * step) / steps);
  }
  touch(target, 'touchend', ...to);
}

let root: HTMLElement;
beforeEach(() => {
  root = document.createElement('div');
  document.body.append(root);
});
afterEach(() => root.remove());

describe('onSwipe', () => {
  it('reports the dominant direction past the threshold only', () => {
    const seen: string[] = [];
    const dispose = onSwipe(root, { onSwipe: (direction) => seen.push(direction) });
    drag(root, [200, 100], [100, 110]);
    drag(root, [100, 100], [110, 200]);
    drag(root, [100, 100], [130, 110]); // too short
    drag(root, [100, 100], [170, 160]); // diagonal: no dominant axis
    expect(seen).toEqual(['left', 'down']);
    dispose();
    drag(root, [200, 100], [100, 100]);
    expect(seen).toHaveLength(2);
  });

  it('honours the edge zone and the enabled guard', () => {
    const seen: string[] = [];
    let enabled = true;
    onSwipe(root, { edge: 24, enabled: () => enabled, onSwipe: (direction) => seen.push(direction) });
    drag(root, [100, 100], [200, 100]); // starts away from the edge
    drag(root, [10, 100], [120, 100]);
    enabled = false;
    drag(root, [10, 100], [120, 100]);
    expect(seen).toEqual(['right']);
  });

  it('ignores gestures that start in text fields', () => {
    const input = document.createElement('input');
    root.append(input);
    const seen: string[] = [];
    onSwipe(root, { onSwipe: (direction) => seen.push(direction) });
    drag(input, [200, 100], [50, 100]);
    expect(seen).toEqual([]);
  });
});

describe('onPullToRefresh', () => {
  it('arms after a long enough downward pull and refreshes on release', () => {
    const pulls: Array<[number, boolean]> = [];
    const onRefresh = vi.fn();
    onPullToRefresh(root, { onPull: (distance, armed) => pulls.push([distance, armed]), onRefresh });
    drag(root, [100, 10], [100, 250]);
    expect(onRefresh).toHaveBeenCalledOnce();
    expect(pulls.some(([, armed]) => armed)).toBe(true);
    expect(pulls.at(-1)).toEqual([0, false]);
  });

  it('does not refresh for a short pull, an upward drag, or a sideways drag', () => {
    const onRefresh = vi.fn();
    onPullToRefresh(root, { onPull: () => {}, onRefresh });
    drag(root, [100, 10], [100, 60]);
    drag(root, [100, 300], [100, 10]);
    drag(root, [10, 10], [250, 40]);
    expect(onRefresh).not.toHaveBeenCalled();
  });

  it('stays inert while a scroller under the finger is scrolled down', () => {
    const list = document.createElement('div');
    root.append(list);
    Object.defineProperty(list, 'scrollTop', { value: 120, configurable: true });
    const onRefresh = vi.fn();
    onPullToRefresh(root, { onPull: () => {}, onRefresh });
    drag(list, [100, 10], [100, 300]);
    expect(onRefresh).not.toHaveBeenCalled();
  });
});

describe('onLongPress', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('fires after holding still and not after moving or lifting early', () => {
    const onPress = vi.fn();
    onLongPress(root, { onLongPress: onPress });

    touch(root, 'touchstart', 50, 50);
    vi.advanceTimersByTime(520);
    expect(onPress).toHaveBeenCalledWith({ x: 50, y: 50 }, root);

    touch(root, 'touchstart', 50, 50);
    touch(root, 'touchmove', 80, 50);
    vi.advanceTimersByTime(520);

    touch(root, 'touchstart', 50, 50);
    touch(root, 'touchend', 50, 50);
    vi.advanceTimersByTime(520);
    expect(onPress).toHaveBeenCalledOnce();
  });
});
