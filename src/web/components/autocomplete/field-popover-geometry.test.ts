import { expect, test } from 'bun:test';

import { createMeasuredPopoverEngine } from './autocomplete-popover-engine.ts';
import {
  AUTOCOMPLETE_MAX_WIDTH,
  placeMeasuredPopover,
  resolvePopupWidth,
} from './autocomplete-position.ts';
import type { AnchorRect, ViewportSize } from './autocomplete-position.ts';

const viewport: ViewportSize = { width: 1280, height: 800 };

function engineHarness(anchor: AnchorRect, viewportSize: ViewportSize, height: number) {
  let currentAnchor: AnchorRect | null = anchor;
  let currentViewport: ViewportSize | null = viewportSize;
  const frames = new Map<number, () => void>();
  let next = 1;
  const engine = createMeasuredPopoverEngine({
    readAnchor: () => currentAnchor,
    readViewport: () => currentViewport,
    measurePopup: (provisionalWidth) => ({ width: provisionalWidth, height }),
    requestFrame: (cb) => {
      const handle = next++;
      frames.set(handle, cb);
      return handle;
    },
    cancelFrame: (handle) => {
      frames.delete(handle);
    },
  });
  const flush = (): void => {
    const pending = [...frames.values()];
    frames.clear();
    for (const cb of pending) cb();
  };
  return {
    engine,
    flush,
    setAnchor: (nextAnchor: AnchorRect | null) => { currentAnchor = nextAnchor; },
    setViewport: (nextViewport: ViewportSize | null) => { currentViewport = nextViewport; },
  };
}

test('narrow field-anchored popup matches the anchor width', () => {
  // No preferredWidth: engine provisional width must equal the anchor.
  const harness = engineHarness({ top: 100, left: 40, bottom: 146, width: 120 }, viewport, 96);
  harness.engine.setOpen(true);
  harness.flush();
  const snapshot = harness.engine.snapshot();
  expect(snapshot.measured).toBe(true);
  expect(snapshot.placement?.width).toBe(120);
  expect(snapshot.placement?.left).toBe(40);
});

test('wide field-anchored popup clamps only to viewport/global maximum', () => {
  const placed = placeMeasuredPopover(
    { top: 100, left: 40, bottom: 146, width: 900 },
    { width: 900, height: 96 },
    viewport,
  );
  expect(placed.width).toBe(AUTOCOMPLETE_MAX_WIDTH);
  // Narrow viewport clamps to available width, never below zero.
  const narrowViewport: ViewportSize = { width: 360, height: 640 };
  expect(resolvePopupWidth(400, narrowViewport)).toBe(360 - 16);
  const narrowPlaced = placeMeasuredPopover(
    { top: 100, left: 40, bottom: 146, width: 400 },
    { width: 400, height: 96 },
    narrowViewport,
  );
  expect(narrowPlaced.width).toBe(360 - 16);
  expect(narrowPlaced.left).toBe(8);
});

test('viewport right/left edges clamp without clipping', () => {
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

test('above/below placement follows the measured height', () => {
  const short = placeMeasuredPopover(
    { top: 634, left: 120, bottom: 680, width: 320 },
    { width: 320, height: 96 },
    viewport,
  );
  expect(short.placement).toBe('below');
  expect(short.top).toBe(686);
  const tall = placeMeasuredPopover(
    { top: 634, left: 120, bottom: 680, width: 320 },
    { width: 320, height: 360 },
    viewport,
  );
  expect(tall.placement).toBe('above');
  expect(tall.top + tall.maxHeight).toBeLessThanOrEqual(634 - 6);
});

test('window resize repositions through the same measured pass', () => {
  const harness = engineHarness({ top: 100, left: 120, bottom: 146, width: 320 }, viewport, 96);
  harness.engine.setOpen(true);
  harness.flush();
  expect(harness.engine.snapshot().placement?.width).toBe(320);
  // Anchor grows (field resized): popup follows the anchor width.
  harness.setAnchor({ top: 100, left: 120, bottom: 146, width: 480 });
  harness.engine.handleLayoutChange();
  harness.flush();
  expect(harness.engine.snapshot().placement?.width).toBe(480);
  // Viewport shrinks: popup clamps to the new available width.
  harness.setViewport({ width: 400, height: 640 });
  harness.engine.handleLayoutChange();
  harness.flush();
  const resized = harness.engine.snapshot().placement;
  expect(resized).not.toBeNull();
  expect(resized!.width).toBeLessThanOrEqual(400 - 16);
  expect(resized!.left).toBeGreaterThanOrEqual(8);
  expect(resized!.left + resized!.width).toBeLessThanOrEqual(400 - 8);
});
