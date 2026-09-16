import { expect, test } from 'bun:test';

import {
  createModalIds,
  FOCUSABLE_SELECTOR,
  isBackdropDismiss,
  isEscapeDismiss,
  MODAL_SIZE_CLASS,
  trapFocusTarget,
} from './modal-logic.ts';

test('modal ids are unique per instance', () => {
  const first = createModalIds();
  const second = createModalIds();
  expect(first.titleId).not.toBe(second.titleId);
  expect(first.descriptionId).not.toBe(second.descriptionId);
  expect(first.titleId).not.toBe(first.descriptionId);
  for (const id of [first.titleId, first.descriptionId, second.titleId, second.descriptionId]) {
    expect(id.length).toBeGreaterThan('ui-modal-'.length);
    expect(id).not.toContain(' ');
  }
});

test('modal size classes cover every size with md as the default card', () => {
  expect(MODAL_SIZE_CLASS.md).toBe('');
  for (const size of ['sm', 'lg', 'xl'] as const) {
    expect(MODAL_SIZE_CLASS[size].startsWith('ui-modal-card--')).toBe(true);
  }
});

test('focusable selector covers interactive elements and skips disabled ones', () => {
  for (const fragment of [
    'button:not([disabled])',
    'input:not([disabled])',
    'textarea:not([disabled])',
    'select:not([disabled])',
    '[href]',
    '[tabindex]:not([tabindex="-1"])',
  ]) {
    expect(FOCUSABLE_SELECTOR).toContain(fragment);
  }
});

test('tab on the last element wraps to the first', () => {
  expect(trapFocusTarget(2, 3, false)).toBe(0);
  expect(trapFocusTarget(0, 1, false)).toBe(0);
});

test('tab in the middle lets the browser move focus natively', () => {
  expect(trapFocusTarget(0, 3, false)).toBeNull();
  expect(trapFocusTarget(1, 3, false)).toBeNull();
});

test('tab with focus outside the dialog moves into the first element', () => {
  expect(trapFocusTarget(-1, 3, false)).toBe(0);
});

test('shift+tab on the first element wraps to the last', () => {
  expect(trapFocusTarget(0, 3, true)).toBe(2);
  expect(trapFocusTarget(0, 1, true)).toBe(0);
  expect(trapFocusTarget(-1, 3, true)).toBe(2);
});

test('shift+tab past the first element keeps native behavior', () => {
  expect(trapFocusTarget(1, 3, true)).toBeNull();
  expect(trapFocusTarget(2, 3, true)).toBeNull();
});

test('focus trap never fires without focusable elements', () => {
  expect(trapFocusTarget(0, 0, false)).toBeNull();
  expect(trapFocusTarget(0, 0, true)).toBeNull();
});

test('backdrop dismiss requires the flag and a direct backdrop hit', () => {
  const backdrop = {};
  expect(isBackdropDismiss(true, backdrop, backdrop)).toBe(true);
  expect(isBackdropDismiss(true, {}, backdrop)).toBe(false);
  expect(isBackdropDismiss(false, backdrop, backdrop)).toBe(false);
});

test('escape dismiss requires the flag and the escape key', () => {
  expect(isEscapeDismiss(true, 'Escape')).toBe(true);
  expect(isEscapeDismiss(true, 'Enter')).toBe(false);
  expect(isEscapeDismiss(false, 'Escape')).toBe(false);
});
