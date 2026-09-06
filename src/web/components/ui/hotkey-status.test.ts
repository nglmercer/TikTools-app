import { describe, expect, test } from 'bun:test';

import { sequenceTriggerHint, summarizeHotkeyStatus } from './hotkey-status.ts';

describe('hotkey status summary', () => {
  test('reports unknown when no backends have reported', () => {
    const summary = summarizeHotkeyStatus({ platform: 'linux', session: 'wayland', backends: [] });
    expect(summary.headline).toBe('Global Hotkeys: status unknown');
    expect(summary.chordsSupported).toBe(false);
    expect(summary.sequencesSupported).toBe(false);
  });

  test('portal-only session supports chords but not sequences', () => {
    const summary = summarizeHotkeyStatus({
      platform: 'linux',
      session: 'wayland',
      backends: [
        {
          backend: 'portal',
          state: 'active',
          detail: '3 shortcuts bound',
          summary: 'Global Hotkeys: active via XDG Desktop Portal (3 shortcuts bound)',
          capabilities: { globalChords: true, arbitraryKeys: false, sequences: false, keyRelease: false },
        },
        {
          backend: 'evdev',
          state: 'permission required',
          detail: 'no readable devices',
          summary: 'Global Hotkeys: permission required via raw input (evdev) (no readable devices)',
          capabilities: { globalChords: true, arbitraryKeys: true, sequences: true, keyRelease: true },
        },
      ],
    });
    expect(summary.headline).toContain('XDG Desktop Portal');
    expect(summary.chordsSupported).toBe(true);
    expect(summary.sequencesSupported).toBe(false);
    expect(summary.needsPermission).toBe(true);
    expect(sequenceTriggerHint(summary, true)).toContain('raw keyboard access');
    expect(sequenceTriggerHint(summary, false)).toBeNull();
  });

  test('x11 session supports sequences without permission prompts', () => {
    const summary = summarizeHotkeyStatus({
      platform: 'linux',
      session: 'x11',
      backends: [
        {
          backend: 'x11',
          state: 'active',
          detail: 'listener attached',
          summary: 'Global Hotkeys: active via X11 (listener attached)',
        },
      ],
    });
    expect(summary.sequencesSupported).toBe(true);
    expect(summary.needsPermission).toBe(false);
    expect(sequenceTriggerHint(summary, true)).toBeNull();
  });

  test('failed backends surface instead of looking alive', () => {
    const summary = summarizeHotkeyStatus({
      platform: 'linux',
      session: 'x11',
      backends: [
        {
          backend: 'x11',
          state: 'failed',
          detail: 'X11 display unavailable',
          summary: 'Global Hotkeys: failed via X11 (X11 display unavailable)',
        },
      ],
    });
    expect(summary.headline).toContain('failed');
    expect(summary.chordsSupported).toBe(false);
    expect(sequenceTriggerHint(summary, true)).toContain('No running backend');
  });
});
