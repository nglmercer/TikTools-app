import { describe, expect, test } from 'vitest';

import { isModifierName, normalizeCapturedKey, normalizeKeyName } from '../../../../../../src/web/entities/automation/lib/key-capture.ts';

function source(overrides: Partial<{ key: string; ctrlKey: boolean; shiftKey: boolean; altKey: boolean; metaKey: boolean }> = {}) {
  return {
    key: '',
    ctrlKey: false,
    shiftKey: false,
    altKey: false,
    metaKey: false,
    ...overrides,
  };
}

describe('shortcut capture', () => {
  test('normalizes browser key names to plugin key names', () => {
    expect(normalizeKeyName(' ')).toBe('space');
    expect(normalizeKeyName('K')).toBe('k');
    expect(normalizeKeyName('Enter')).toBe('enter');
    expect(normalizeKeyName('Escape')).toBe('esc');
    expect(normalizeKeyName('ArrowUp')).toBe('up');
    expect(normalizeKeyName('F12')).toBe('f12');
    expect(normalizeKeyName('Control')).toBe('ctrl');
    expect(normalizeKeyName('Dead')).toBe('');
    expect(normalizeKeyName('')).toBe('');
  });

  test('captures chords with modifiers', () => {
    expect(normalizeCapturedKey(source({ key: 'k', ctrlKey: true }))).toEqual({ key: 'k', modifiers: 'ctrl' });
    expect(normalizeCapturedKey(source({ key: 'K', ctrlKey: true, shiftKey: true }))).toEqual({ key: 'k', modifiers: 'ctrl+shift' });
    expect(normalizeCapturedKey(source({ key: ' ' }))).toEqual({ key: 'space', modifiers: '' });
  });

  test('stays armed on bare modifiers and ignored keys', () => {
    expect(normalizeCapturedKey(source({ key: 'Control', ctrlKey: true }))).toBeNull();
    expect(normalizeCapturedKey(source({ key: 'Shift', shiftKey: true }))).toBeNull();
    expect(normalizeCapturedKey(source({ key: 'Dead' }))).toBeNull();
    expect(isModifierName('ctrl')).toBe(true);
    expect(isModifierName('k')).toBe(false);
  });
});
