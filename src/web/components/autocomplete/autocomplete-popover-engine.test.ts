import { expect, test } from 'bun:test';

import { createMeasuredPopoverEngine } from './autocomplete-popover-engine.ts';
import { AUTOCOMPLETE_PREFERRED_WIDTH, type AnchorRect, type PopupSize, type ViewportSize } from './autocomplete-position.ts';

function setup() {
  let anchor: AnchorRect | null = { top: 634, left: 120, bottom: 680, width: 320 };
  let viewport: ViewportSize | null = { width: 1280, height: 800 };
  let size: PopupSize | null = { width: 320, height: 96 };
  const frames = new Map<number, () => void>();
  let nextHandle = 1;
  let measureCalls = 0;
  const engine = createMeasuredPopoverEngine({
    readAnchor: () => anchor,
    readViewport: () => viewport,
    measurePopup: (provisionalWidth) => {
      measureCalls += 1;
      return size ? { width: provisionalWidth, height: size.height } : null;
    },
    requestFrame: (callback) => {
      const handle = nextHandle;
      nextHandle += 1;
      frames.set(handle, callback);
      return handle;
    },
    cancelFrame: (handle) => {
      frames.delete(handle);
    },
  });
  const flush = (): void => {
    const pending = [...frames.values()];
    frames.clear();
    for (const callback of pending) callback();
  };
  return {
    engine,
    flush,
    setAnchor: (next: AnchorRect | null) => { anchor = next; },
    setViewport: (next: ViewportSize | null) => { viewport = next; },
    setSize: (next: PopupSize | null) => { size = next; },
    measureCalls: () => measureCalls,
    pendingFrames: () => frames.size,
  };
}

test('first open stays hidden until a real measurement places it', () => {
  const harness = setup();
  harness.engine.setOpen(true);
  expect(harness.engine.snapshot()).toMatchObject({ open: true, measured: false, placement: null });
  harness.flush();
  const snapshot = harness.engine.snapshot();
  expect(snapshot.measured).toBe(true);
  expect(snapshot.placement?.placement).toBe('below');
  expect(snapshot.placement?.top).toBe(686);
});

test('layout signals collapse into one measure-then-place pass', () => {
  const harness = setup();
  harness.engine.setOpen(true);
  harness.engine.handleLayoutChange();
  harness.engine.handleLayoutChange();
  harness.engine.handleLayoutChange();
  expect(harness.pendingFrames()).toBe(1);
  harness.flush();
  expect(harness.measureCalls()).toBe(1);
  expect(harness.engine.snapshot().measured).toBe(true);
});

test('an anchor move repositions through the same measured pass', () => {
  const harness = setup();
  harness.engine.setOpen(true);
  harness.flush();
  expect(harness.engine.snapshot().placement?.left).toBe(120);
  harness.setAnchor({ top: 634, left: 400, bottom: 680, width: 320 });
  harness.engine.handleLayoutChange();
  harness.flush();
  expect(harness.engine.snapshot().placement?.left).toBe(400);
  expect(harness.engine.snapshot().measured).toBe(true);
});

test('a failed measurement stays hidden and recovers on the next signal', () => {
  const harness = setup();
  harness.setSize(null);
  harness.engine.setOpen(true);
  harness.flush();
  expect(harness.engine.snapshot()).toMatchObject({ measured: false, placement: null });
  harness.setSize({ width: 320, height: 96 });
  harness.engine.handleLayoutChange();
  harness.flush();
  expect(harness.engine.snapshot().measured).toBe(true);
  expect(harness.engine.snapshot().placement?.placement).toBe('below');
});

test('filtering down to one row keeps the popup below on its real height', () => {
  const harness = setup();
  harness.setSize({ width: 320, height: 360 });
  harness.engine.setOpen(true);
  harness.flush();
  expect(harness.engine.snapshot().placement?.placement).toBe('above');
  harness.setSize({ width: 320, height: 96 });
  harness.engine.handleLayoutChange();
  harness.flush();
  const snapshot = harness.engine.snapshot();
  expect(snapshot.placement?.placement).toBe('below');
  expect(snapshot.placement?.top).toBe(686);
});

test('closing resets the placement and cancels the pending frame', () => {
  const harness = setup();
  harness.engine.setOpen(true);
  expect(harness.pendingFrames()).toBe(1);
  harness.engine.setOpen(false);
  expect(harness.pendingFrames()).toBe(0);
  expect(harness.engine.snapshot()).toMatchObject({ open: false, measured: false, placement: null });
  harness.flush();
  expect(harness.measureCalls()).toBe(0);
});

test('option changes reposition with the new width budget', () => {
  const harness = setup();
  harness.engine.setOpen(true);
  harness.flush();
  expect(harness.engine.snapshot().placement?.width).toBe(320);
  harness.engine.setOptions({ preferredWidth: 420, maxWidth: 420 });
  harness.flush();
  expect(harness.engine.snapshot().placement?.width).toBe(420);
});

test('caret anchors size from the preferred width, not the zero-width caret rect', () => {
  const harness = setup();
  harness.setAnchor({ top: 204, left: 184, bottom: 222, width: 2 });
  harness.engine.setOptions({ anchorMode: 'caret' });
  harness.engine.setOpen(true);
  harness.flush();
  expect(harness.engine.snapshot().placement?.width).toBe(AUTOCOMPLETE_PREFERRED_WIDTH);
});

test('field anchors keep matching their control width', () => {
  const harness = setup();
  harness.engine.setOptions({ anchorMode: 'field' });
  harness.engine.setOpen(true);
  harness.flush();
  expect(harness.engine.snapshot().placement?.width).toBe(320);
});

test('subscribers only fire on real snapshot changes', () => {
  const harness = setup();
  let notifications = 0;
  const unsubscribe = harness.engine.subscribe(() => {
    notifications += 1;
  });
  harness.engine.setOpen(true);
  harness.flush();
  const afterOpen = notifications;
  expect(afterOpen).toBeGreaterThan(0);
  harness.engine.handleLayoutChange();
  harness.flush();
  expect(notifications).toBe(afterOpen);
  unsubscribe();
});
