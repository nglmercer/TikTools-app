import { describe, expect, test } from 'bun:test';

import {
  isWidgetTestHookEnabled,
  resolveWidgetTestHookEnabled,
  WIDGET_TEST_HOOK_ENV,
} from './test-hook.ts';

describe('widget test hook flag', () => {
  test('exposes the documented E2E env name', () => {
    expect(WIDGET_TEST_HOOK_ENV).toBe('VITE_TIKTOOLS_WIDGET_TEST_HOOK');
  });

  test('production/default env does not install the hook', () => {
    expect(resolveWidgetTestHookEnabled({})).toBe(false);
    expect(resolveWidgetTestHookEnabled({ dev: false })).toBe(false);
    expect(resolveWidgetTestHookEnabled({ dev: false, testHookFlag: undefined })).toBe(false);
    expect(resolveWidgetTestHookEnabled({ dev: false, testHookFlag: '0' })).toBe(false);
    expect(resolveWidgetTestHookEnabled({ dev: false, testHookFlag: '' })).toBe(false);
    expect(resolveWidgetTestHookEnabled({ dev: false, testHookFlag: false })).toBe(false);
    // Bun has no Vite defines, so the live check must also default off.
    expect(isWidgetTestHookEnabled()).toBe(false);
  });

  test('user-controlled URL values never enable the hook', () => {
    for (const flag of ['?testHook=1', '#testHook=1', 'testHook=1', 'true', 'yes', '1 ']) {
      expect(resolveWidgetTestHookEnabled({ dev: false, testHookFlag: flag })).toBe(false);
    }
  });

  test('explicit E2E build flag and dev mode enable the hook', () => {
    expect(resolveWidgetTestHookEnabled({ dev: false, testHookFlag: '1' })).toBe(true);
    expect(resolveWidgetTestHookEnabled({ dev: false, testHookFlag: true })).toBe(true);
    expect(resolveWidgetTestHookEnabled({ dev: true })).toBe(true);
  });
});
