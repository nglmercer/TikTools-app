import { expect, test } from 'bun:test';

import {
  AUTOCOMPLETE_MAX_WIDTH,
  computePopoverPosition,
} from './autocomplete-position.ts';

const viewport = { width: 1280, height: 800 };

test('opens below the anchor with the input width', () => {
  const placed = computePopoverPosition(
    { top: 100, left: 120, bottom: 146, width: 320 },
    viewport,
  );
  expect(placed.placement).toBe('below');
  expect(placed.top).toBe(152);
  expect(placed.left).toBe(120);
  expect(placed.width).toBe(320);
});

test('flips above when there is no room below', () => {
  const placed = computePopoverPosition(
    { top: 700, left: 120, bottom: 746, width: 320 },
    viewport,
  );
  expect(placed.placement).toBe('above');
  expect(placed.top + placed.maxHeight).toBeLessThanOrEqual(700);
  expect(placed.top).toBeGreaterThanOrEqual(8);
});

test('width never drops below the input width nor above 520', () => {
  const wide = computePopoverPosition({ top: 100, left: 40, bottom: 146, width: 900 }, viewport);
  expect(wide.width).toBe(AUTOCOMPLETE_MAX_WIDTH);
  const narrow = computePopoverPosition({ top: 100, left: 40, bottom: 146, width: 200 }, viewport);
  expect(narrow.width).toBe(200);
});

test('narrow screens shrink the popover to the viewport margins', () => {
  const placed = computePopoverPosition(
    { top: 100, left: 10, bottom: 146, width: 400 },
    { width: 360, height: 640 },
  );
  expect(placed.width).toBe(360 - 16);
  expect(placed.left).toBe(8);
  expect(placed.left + placed.width).toBeLessThanOrEqual(360 - 8);
});

test('left edge clamps into the viewport (never clipped)', () => {
  const right = computePopoverPosition(
    { top: 100, left: 1200, bottom: 146, width: 320 },
    viewport,
  );
  expect(right.left + right.width).toBeLessThanOrEqual(1280 - 8);
  const left = computePopoverPosition(
    { top: 100, left: -40, bottom: 146, width: 320 },
    viewport,
  );
  expect(left.left).toBe(8);
});

test('short viewports mirror the min(360px, 50vh) cap', () => {
  const placed = computePopoverPosition(
    { top: 200, left: 120, bottom: 246, width: 320 },
    { width: 1280, height: 400 },
  );
  expect(placed.maxHeight).toBe(200);
});

test('custom gap and margin options are honored', () => {
  const placed = computePopoverPosition(
    { top: 100, left: -40, bottom: 146, width: 320 },
    viewport,
    { gap: 12, margin: 20 },
  );
  expect(placed.top).toBe(158);
  expect(placed.left).toBe(20);
});

test('custom maxWidth caps wide anchors', () => {
  const placed = computePopoverPosition(
    { top: 100, left: 40, bottom: 146, width: 900 },
    viewport,
    { maxWidth: 200 },
  );
  expect(placed.width).toBe(200);
  expect(placed.left).toBe(40);
});

test('custom maxHeight still obeys the 50vh rule', () => {
  const placed = computePopoverPosition(
    { top: 100, left: 40, bottom: 146, width: 320 },
    { width: 1280, height: 400 },
    { maxHeight: 500 },
  );
  expect(placed.maxHeight).toBe(200);
});

test('stays below and clamps when neither side fits', () => {
  const placed = computePopoverPosition(
    { top: 90, left: 120, bottom: 110, width: 320 },
    { width: 1280, height: 200 },
  );
  expect(placed.placement).toBe('below');
  expect(placed.maxHeight).toBe(100);
  expect(placed.top).toBe(92);
  expect(placed.top + placed.maxHeight).toBeLessThanOrEqual(200 - 8);
});
