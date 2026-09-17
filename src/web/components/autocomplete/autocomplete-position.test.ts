import { expect, test } from 'bun:test';

import {
  AUTOCOMPLETE_MAX_WIDTH,
  placeMeasuredPopover,
  resolvePopoverVisibility,
  resolvePopupWidth,
} from './autocomplete-position.ts';

const viewport = { width: 1280, height: 800 };

test('a one-row popup near the bottom stays below on its real height', () => {
  // A 360px budget would flip here (686 + 360 > 792); the measured 96px
  // panel fits below and must not jump above the input.
  const placed = placeMeasuredPopover(
    { top: 634, left: 120, bottom: 680, width: 320 },
    { width: 320, height: 96 },
    viewport,
  );
  expect(placed.placement).toBe('below');
  expect(placed.top).toBe(686);
  expect(placed.left).toBe(120);
  expect(placed.width).toBe(320);
  expect(placed.maxHeight).toBe(96);
});

test('the same anchor flips only when the real height needs it', () => {
  const short = placeMeasuredPopover(
    { top: 634, left: 120, bottom: 680, width: 320 },
    { width: 320, height: 96 },
    viewport,
  );
  const tall = placeMeasuredPopover(
    { top: 634, left: 120, bottom: 680, width: 320 },
    { width: 320, height: 360 },
    viewport,
  );
  expect(short.placement).toBe('below');
  expect(tall.placement).toBe('above');
  expect(tall.top + tall.maxHeight).toBeLessThanOrEqual(634 - 6);
  expect(tall.top).toBeGreaterThanOrEqual(8);
});

test('placement never intersects the anchor on either side', () => {
  const gap = 6;
  const cases = [
    placeMeasuredPopover({ top: 100, left: 120, bottom: 146, width: 320 }, { width: 320, height: 200 }, viewport),
    placeMeasuredPopover({ top: 634, left: 120, bottom: 680, width: 320 }, { width: 320, height: 360 }, viewport),
    placeMeasuredPopover({ top: 30, left: 120, bottom: 50, width: 320 }, { width: 320, height: 300 }, { width: 1280, height: 120 }),
    placeMeasuredPopover({ top: 100, left: 120, bottom: 120, width: 320 }, { width: 320, height: 300 }, { width: 1280, height: 200 }),
  ];
  const anchors = [
    { top: 100, bottom: 146 },
    { top: 634, bottom: 680 },
    { top: 30, bottom: 50 },
    { top: 100, bottom: 120 },
  ];
  for (const [index, placed] of cases.entries()) {
    const anchor = anchors[index]!;
    if (placed.placement === 'below') {
      expect(placed.top).toBeGreaterThanOrEqual(anchor.bottom + gap);
    } else {
      expect(placed.top + placed.maxHeight).toBeLessThanOrEqual(anchor.top - gap);
    }
  }
});

test('a constrained below popup is never clamped upward through the anchor', () => {
  // Only 56px below, 16px above: below wins with a 56px budget and must
  // start at the anchor edge (56), not slide up to fit the budget (52).
  const placed = placeMeasuredPopover(
    { top: 30, left: 120, bottom: 50, width: 320 },
    { width: 320, height: 300 },
    { width: 1280, height: 120 },
  );
  expect(placed.placement).toBe('below');
  expect(placed.top).toBe(56);
  expect(placed.maxHeight).toBe(56);
});

test('when neither side fits, the roomier side wins with its exact space', () => {
  const placed = placeMeasuredPopover(
    { top: 100, left: 120, bottom: 120, width: 320 },
    { width: 320, height: 300 },
    { width: 1280, height: 200 },
  );
  expect(placed.placement).toBe('above');
  expect(placed.maxHeight).toBe(86);
  expect(placed.top).toBe(8);
  expect(placed.top + placed.maxHeight).toBe(100 - 6);
});

test('width follows the measured panel, clamped to viewport and options', () => {
  const caret = placeMeasuredPopover(
    { top: 100, left: 120, bottom: 118, width: 2 },
    { width: 360, height: 96 },
    viewport,
  );
  expect(caret.width).toBe(360);
  expect(caret.left).toBe(120);
  const wide = placeMeasuredPopover(
    { top: 100, left: 40, bottom: 146, width: 900 },
    { width: 900, height: 96 },
    viewport,
  );
  expect(wide.width).toBe(AUTOCOMPLETE_MAX_WIDTH);
  const narrow = placeMeasuredPopover(
    { top: 100, left: 40, bottom: 146, width: 400 },
    { width: 400, height: 96 },
    { width: 360, height: 640 },
  );
  expect(narrow.width).toBe(360 - 16);
  expect(narrow.left).toBe(8);
});

test('resolvePopupWidth honors min/max options and narrow screens', () => {
  expect(resolvePopupWidth(360, viewport, { minWidth: 320, maxWidth: 420 })).toBe(360);
  expect(resolvePopupWidth(200, viewport, { minWidth: 320, maxWidth: 420 })).toBe(320);
  expect(resolvePopupWidth(900, viewport, { maxWidth: 420 })).toBe(420);
  expect(resolvePopupWidth(400, { width: 360, height: 640 }, { minWidth: 320 })).toBe(360 - 16);
});

test('left edge clamps into the viewport (never clipped)', () => {
  const right = placeMeasuredPopover(
    { top: 100, left: 1200, bottom: 146, width: 320 },
    { width: 320, height: 96 },
    viewport,
  );
  expect(right.left + right.width).toBeLessThanOrEqual(1280 - 8);
  const left = placeMeasuredPopover(
    { top: 100, left: -40, bottom: 146, width: 320 },
    { width: 320, height: 96 },
    viewport,
  );
  expect(left.left).toBe(8);
});

test('tall panels mirror the min(360px, 50vh) cap', () => {
  const placed = placeMeasuredPopover(
    { top: 100, left: 120, bottom: 146, width: 320 },
    { width: 320, height: 1000 },
    { width: 1280, height: 400 },
  );
  expect(placed.placement).toBe('below');
  expect(placed.maxHeight).toBe(200);
});

test('custom gap, margin, and width options are honored', () => {
  const placed = placeMeasuredPopover(
    { top: 100, left: -40, bottom: 146, width: 320 },
    { width: 320, height: 96 },
    viewport,
    { gap: 12, margin: 20, minWidth: 200, maxWidth: 400 },
  );
  expect(placed.top).toBe(158);
  expect(placed.left).toBe(20);
  expect(placed.width).toBe(320);
});

test('the popup stays hidden until a measurement places it', () => {
  const placement = placeMeasuredPopover(
    { top: 100, left: 120, bottom: 146, width: 320 },
    { width: 320, height: 96 },
    viewport,
  );
  expect(resolvePopoverVisibility({ open: true, measured: false, placement: null })).toBe('hidden');
  expect(resolvePopoverVisibility({ open: true, measured: false, placement })).toBe('hidden');
  expect(resolvePopoverVisibility({ open: false, measured: true, placement })).toBe('hidden');
  expect(resolvePopoverVisibility({ open: true, measured: true, placement: null })).toBe('hidden');
  expect(resolvePopoverVisibility({ open: true, measured: true, placement })).toBe('visible');
});
