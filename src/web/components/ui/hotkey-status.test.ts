import { describe, expect, test } from 'bun:test';

import type { HotkeyStatusData } from '../../../shared/messages.ts';
import { formatHotkeyChord, hotkeyListenerState, hotkeyNeedsSeatAccess } from './hotkey-status.ts';

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

describe('hotkeyNeedsSeatAccess', () => {
  function withDetail(state: string, detail: string): HotkeyStatusData {
    return {
      platform: 'linux',
      session: 'wayland',
      backends: [{ backend: 'rdev', state, detail, summary: `${state}: ${detail}` }],
    };
  }

  test('matches only /dev/input denials on failed or permission states', () => {
    const denied = 'Wayland global capture requires read access to /dev/input/event* (15 denied).';
    expect(hotkeyNeedsSeatAccess(withDetail('failed', denied))).toBe(true);
    expect(hotkeyNeedsSeatAccess(withDetail('permission-required', denied))).toBe(true);
    expect(hotkeyNeedsSeatAccess(withDetail('failed', 'cannot open display :0'))).toBe(false);
    expect(hotkeyNeedsSeatAccess(withDetail('running', denied))).toBe(false);
    expect(hotkeyNeedsSeatAccess(withDetail('failed', ''))).toBe(false);
    expect(hotkeyNeedsSeatAccess(null)).toBe(false);
    expect(hotkeyNeedsSeatAccess({ platform: '', session: '', backends: [] })).toBe(false);
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
