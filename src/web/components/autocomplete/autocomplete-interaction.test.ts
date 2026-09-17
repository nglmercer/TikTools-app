import { expect, test } from 'bun:test';

import {
  createAutocompleteController,
  type AutocompleteControllerSnapshot,
} from './autocomplete-controller.ts';
import { createAutocompleteInteraction } from './autocomplete-interaction.ts';

const PRESETS = [
  { id: 'local-node', label: 'localhost:3000', url: 'http://localhost:3000/' },
  { id: 'local-py', label: '127.0.0.1:8000', url: 'http://127.0.0.1:8000/' },
];

function setup() {
  const frames = new Map<number, () => void>();
  let nextHandle = 1;
  const settled: AutocompleteControllerSnapshot[] = [];
  const session = createAutocompleteInteraction({
    controller: createAutocompleteController({ mode: 'preset', presets: PRESETS }),
    schedule: {
      requestFrame: (callback) => {
        const handle = nextHandle;
        nextHandle += 1;
        frames.set(handle, callback);
        return handle;
      },
      cancelFrame: (handle) => {
        frames.delete(handle);
      },
    },
    onSettled: (snapshot) => {
      settled.push(snapshot);
    },
  });
  const flush = (): void => {
    const pending = [...frames.values()];
    frames.clear();
    for (const callback of pending) callback();
  };
  return { session, flush, settled, pendingFrames: () => frames.size };
}

test('focus URL opens presets and a rerender keeps them open', () => {
  const harness = setup();
  const opened = harness.session.update({ value: '', caret: 0, focused: true });
  expect(opened.open).toBe(true);
  expect(opened.rowCount).toBe(2);
  const rerendered = harness.session.rerender();
  expect(rerendered.open).toBe(true);
  expect(rerendered.rowCount).toBe(2);
  expect(harness.session.interactionSnapshot()).toMatchObject({ inputFocused: true, blurDeferred: false });
});

test('a row pointer press defers blur, then the commit closes once', () => {
  const harness = setup();
  harness.session.update({ value: '', caret: 0, focused: true });
  harness.session.beginPointerSelection();
  // Touch/WebView blur racing the tap: the popup must survive the frame.
  const duringBlur = harness.session.update({ value: '', caret: 0, focused: false });
  expect(duringBlur.open).toBe(true);
  expect(harness.session.interactionSnapshot()).toMatchObject({ pointerSelecting: true, blurDeferred: true });
  harness.flush();
  expect(harness.session.snapshot().open).toBe(true);
  const committed = harness.session.commit('', 0);
  expect(committed?.value).toBe('http://localhost:3000/');
  expect(harness.session.snapshot().open).toBe(false);
  expect(harness.session.interactionSnapshot()).toMatchObject({
    pointerSelecting: false,
    blurDeferred: false,
  });
});

test('an external blur closes after the one-frame grace period', () => {
  const harness = setup();
  harness.session.update({ value: '', caret: 0, focused: true });
  const duringBlur = harness.session.update({ value: '', caret: 0, focused: false });
  expect(duringBlur.open).toBe(true);
  expect(harness.pendingFrames()).toBe(1);
  harness.flush();
  expect(harness.session.snapshot().open).toBe(false);
  expect(harness.settled.at(-1)?.open).toBe(false);
});

test('pointer presence over the popup survives blur until it leaves', () => {
  const harness = setup();
  harness.session.update({ value: '', caret: 0, focused: true });
  harness.session.setPopupPointerInside(true);
  harness.session.update({ value: '', caret: 0, focused: false });
  harness.flush();
  expect(harness.session.snapshot().open).toBe(true);
  const afterLeave = harness.session.setPopupPointerInside(false);
  expect(afterLeave.open).toBe(false);
  expect(harness.session.interactionSnapshot().blurDeferred).toBe(false);
});

test('refocusing during the grace period cancels the pending blur', () => {
  const harness = setup();
  harness.session.update({ value: '', caret: 0, focused: true });
  harness.session.update({ value: '', caret: 0, focused: false });
  expect(harness.pendingFrames()).toBe(1);
  const refocused = harness.session.update({ value: '', caret: 0, focused: true });
  expect(refocused.open).toBe(true);
  expect(harness.pendingFrames()).toBe(0);
  harness.flush();
  expect(harness.session.snapshot().open).toBe(true);
  expect(harness.settled).toHaveLength(0);
});

test('ending a press without committing leaves a focused popup open', () => {
  const harness = setup();
  harness.session.update({ value: '', caret: 0, focused: true });
  harness.session.beginPointerSelection();
  harness.session.endPointerSelection();
  expect(harness.session.snapshot().open).toBe(true);
  expect(harness.session.interactionSnapshot().pointerSelecting).toBe(false);
});

test('Escape clears pointer state and closes explicitly', () => {
  const harness = setup();
  harness.session.update({ value: '', caret: 0, focused: true });
  harness.session.beginPointerSelection();
  harness.session.setPopupPointerInside(true);
  expect(harness.session.key('Escape')).toBe('dismissed');
  expect(harness.session.snapshot().open).toBe(false);
  expect(harness.session.interactionSnapshot()).toMatchObject({
    pointerSelecting: false,
    popupPointerInside: false,
    blurDeferred: false,
  });
});

test('a press that ends outside the rows cannot wedge the popup open', () => {
  const harness = setup();
  harness.session.update({ value: '', caret: 0, focused: true });
  harness.session.beginPointerSelection();
  // Drag off the panel: no row pointerup ever arrives, but leaving the
  // popup ends the press, so a later blur still closes.
  harness.session.setPopupPointerInside(false);
  expect(harness.session.interactionSnapshot().pointerSelecting).toBe(false);
  harness.session.update({ value: '', caret: 0, focused: false });
  harness.flush();
  expect(harness.session.snapshot().open).toBe(false);
});

test('zero results after editing still close the popup', () => {
  const harness = setup();
  harness.session.update({ value: '', caret: 0, focused: true });
  const edited = harness.session.update({ value: 'https://no-such-preset.invalid/', caret: 31, focused: true });
  expect(edited.open).toBe(false);
  expect(edited.rowCount).toBe(0);
});
