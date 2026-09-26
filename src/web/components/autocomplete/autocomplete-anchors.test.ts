import { expect, test } from 'bun:test';

import { caretAnchorFromMarker, resolveAnchorRect, resolveDesiredPopupWidth } from './autocomplete-anchors.ts';
import { AUTOCOMPLETE_PREFERRED_WIDTH } from './autocomplete-position.ts';

test('mirror metrics become a thin viewport caret rectangle', () => {
  const anchor = caretAnchorFromMarker({
    inputLeft: 120,
    inputTop: 200,
    markerLeft: 64,
    markerTop: 4,
    markerHeight: 18,
    scrollLeft: 0,
    scrollTop: 0,
  });
  expect(anchor).toEqual({ top: 204, left: 184, bottom: 222, width: 2 });
});

test('the virtual anchor moves when the caret moves', () => {
  const before = caretAnchorFromMarker({
    inputLeft: 120,
    inputTop: 200,
    markerLeft: 64,
    markerTop: 4,
    markerHeight: 18,
    scrollLeft: 0,
    scrollTop: 0,
  });
  const after = caretAnchorFromMarker({
    inputLeft: 120,
    inputTop: 200,
    markerLeft: 96,
    markerTop: 4,
    markerHeight: 18,
    scrollLeft: 0,
    scrollTop: 0,
  });
  expect(after.left).toBeGreaterThan(before.left);
  expect(after.top).toBe(before.top);
  expect(after.bottom - after.top).toBe(18);
});

test('input scrolling shifts the caret anchor with the text', () => {
  const unscrolled = caretAnchorFromMarker({
    inputLeft: 120,
    inputTop: 200,
    markerLeft: 400,
    markerTop: 4,
    markerHeight: 18,
    scrollLeft: 0,
    scrollTop: 0,
  });
  const scrolled = caretAnchorFromMarker({
    inputLeft: 120,
    inputTop: 200,
    markerLeft: 400,
    markerTop: 4,
    markerHeight: 18,
    scrollLeft: 120,
    scrollTop: 0,
  });
  expect(scrolled.left).toBe(unscrolled.left - 120);
});

test('zero-height markers still produce a usable line anchor', () => {
  const anchor = caretAnchorFromMarker({
    inputLeft: 120,
    inputTop: 200,
    markerLeft: 64,
    markerTop: 4,
    markerHeight: 0,
    scrollLeft: 0,
    scrollTop: 0,
  });
  expect(anchor.bottom - anchor.top).toBe(1);
});

test('caret mode prefers the caret rect and falls back to the field box', () => {
  const field = { top: 100, left: 120, bottom: 146, width: 320 };
  const caret = { top: 104, left: 184, bottom: 122, width: 2 };
  expect(resolveAnchorRect({ mode: 'caret', field, caret })).toBe(caret);
  expect(resolveAnchorRect({ mode: 'caret', field, caret: null })).toBe(field);
  expect(resolveAnchorRect({ mode: 'caret', field: null, caret: null })).toBeNull();
});

test('caret popovers size from the preferred width, not the zero-width point', () => {
  expect(resolveDesiredPopupWidth({ anchorWidth: 2, mode: 'caret' })).toBe(AUTOCOMPLETE_PREFERRED_WIDTH);
  expect(resolveDesiredPopupWidth({ anchorWidth: 2, mode: 'caret', preferredWidth: 420 })).toBe(420);
});

test('field popovers match their control width unless overridden', () => {
  expect(resolveDesiredPopupWidth({ anchorWidth: 320, mode: 'field' })).toBe(320);
  expect(resolveDesiredPopupWidth({ anchorWidth: 320, mode: 'field', preferredWidth: 420 })).toBe(420);
});

test('field mode always uses the control box', () => {
  const field = { top: 100, left: 120, bottom: 146, width: 320 };
  const caret = { top: 104, left: 184, bottom: 122, width: 2 };
  expect(resolveAnchorRect({ mode: 'field', field, caret })).toBe(field);
  expect(resolveAnchorRect({ mode: 'field', field: null, caret })).toBeNull();
});
