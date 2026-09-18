import { describe, expect, test } from 'bun:test';

import type { HotkeyStatusData } from '../../../shared/messages.ts';
import { formatHotkeyChord, hotkeyListenerState } from './hotkey-status.ts';

function status(states: string[]): HotkeyStatusData {
  return {
    platform: 'windows',
    session: 'n/a',
    backends: states.map((state, index) => ({
      backend: `backend-${index}`,
      state,
      detail: '',
      summary: `Global Hotkeys: ${state} via backend-${index}`,
    })),
  };
}

describe('hotkeyListenerState', () => {
  test('classifies each listener condition', () => {
    expect(hotkeyListenerState(status(['active']))).toBe('active');
    expect(hotkeyListenerState(status(['starting']))).toBe('starting');
    expect(hotkeyListenerState(status(['permission required']))).toBe('permission');
    expect(hotkeyListenerState(status(['failed']))).toBe('failed');
    expect(hotkeyListenerState(status(['unsupported']))).toBe('unsupported');
    expect(hotkeyListenerState(null)).toBe('unknown');
    expect(hotkeyListenerState({ platform: '', session: '', backends: [] })).toBe('unknown');
  });

  test('permission and failure outrank a healthy backend', () => {
    expect(hotkeyListenerState(status(['active', 'permission required']))).toBe('permission');
    expect(hotkeyListenerState(status(['active', 'failed']))).toBe('failed');
  });
});

describe('formatHotkeyChord', () => {
  test('renders canonical chords', () => {
    expect(formatHotkeyChord('k', 'ctrl')).toBe('Ctrl+K');
    expect(formatHotkeyChord('k', 'ctrl+shift')).toBe('Ctrl+Shift+K');
    expect(formatHotkeyChord('a', '')).toBe('A');
    expect(formatHotkeyChord('space', 'alt')).toBe('Alt+Space');
  });
});
