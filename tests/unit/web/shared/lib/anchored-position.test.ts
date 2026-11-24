import { describe, expect, it } from 'vitest';
import { computeAnchoredPosition, type Rect } from './anchored-position.ts';

const viewport = { width: 800, height: 600 };

function rect(left: number, top: number, width: number, height: number): Rect {
  return { left, top, width, height, right: left + width, bottom: top + height };
}

describe('computeAnchoredPosition', () => {
  it('places below the anchor aligned to its start', () => {
    expect(computeAnchoredPosition(rect(100, 100, 120, 26), { width: 160, height: 200 }, viewport, 'bottom-start', 2)).toEqual({
      top: 128,
      left: 100,
      placement: 'bottom-start',
    });
  });

  it('flips above when there is no room below', () => {
    expect(computeAnchoredPosition(rect(100, 500, 120, 26), { width: 160, height: 200 }, viewport, 'bottom-start', 2)).toEqual({
      top: 298,
      left: 100,
      placement: 'top-start',
    });
  });

  it('opens submenus to the left when the right side overflows', () => {
    expect(computeAnchoredPosition(rect(600, 100, 180, 24), { width: 180, height: 120 }, viewport, 'right-start')).toEqual({
      top: 100,
      left: 420,
      placement: 'left-start',
    });
  });

  it('clamps inside the viewport when neither side fits', () => {
    const position = computeAnchoredPosition(rect(700, 300, 80, 26), { width: 200, height: 590 }, viewport, 'bottom-start');
    expect(position.left).toBe(600);
    expect(position.top).toBe(10);
  });
});
