import { expect, test } from 'bun:test';

import { computeTooltipPosition, type TooltipRect, type TooltipViewport } from './tooltip-logic.ts';

const VIEWPORT: TooltipViewport = { width: 1280, height: 800 };

function triggerAt(top: number, left: number, width = 40, height = 24): TooltipRect {
  return { top, left, width, height };
}

test('centers the tooltip on the preferred side', () => {
  const above = computeTooltipPosition(triggerAt(400, 600), { width: 120, height: 32 }, 'top', VIEWPORT);
  expect(above.position).toBe('top');
  expect(above.top).toBe(400 - 32 - 8);
  expect(above.left).toBe(600 + 20 - 60);

  const below = computeTooltipPosition(triggerAt(400, 600), { width: 120, height: 32 }, 'bottom', VIEWPORT);
  expect(below.position).toBe('bottom');
  expect(below.top).toBe(400 + 24 + 8);

  const right = computeTooltipPosition(triggerAt(400, 600), { width: 120, height: 32 }, 'right', VIEWPORT);
  expect(right.position).toBe('right');
  expect(right.left).toBe(600 + 40 + 8);
  expect(right.top).toBe(400 + 12 - 16);
});

test('flips to the opposite side on overflow', () => {
  const nearTop = computeTooltipPosition(triggerAt(4, 600), { width: 120, height: 32 }, 'top', VIEWPORT);
  expect(nearTop.position).toBe('bottom');

  const nearBottom = computeTooltipPosition(triggerAt(770, 600), { width: 120, height: 32 }, 'bottom', VIEWPORT);
  expect(nearBottom.position).toBe('top');

  const nearLeft = computeTooltipPosition(triggerAt(400, 4), { width: 120, height: 32 }, 'left', VIEWPORT);
  expect(nearLeft.position).toBe('right');

  const nearRight = computeTooltipPosition(triggerAt(400, 1240), { width: 120, height: 32 }, 'right', VIEWPORT);
  expect(nearRight.position).toBe('left');
});

test('clamps into the viewport when no side fits', () => {
  const huge = computeTooltipPosition(triggerAt(400, 600), { width: 2000, height: 1200 }, 'top', VIEWPORT);
  expect(huge.top).toBe(8);
  expect(huge.left).toBe(8);
  expect(huge.position).toBe('top');
});
