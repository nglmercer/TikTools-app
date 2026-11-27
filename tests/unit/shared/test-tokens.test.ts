import { describe, expect, it } from 'vitest';
import * as tokens from '../../shared/index.ts';
import { LOCALE_STORAGE_KEY } from '@/shared/i18n/locale-preference.ts';
import { VIEW_STATE_STORAGE_KEY } from '@/shared/lib/view-state.ts';

describe('shared test tokens', () => {
  it('pins the storage keys to the ones the app writes', () => {
    expect(tokens.STORAGE_KEY.locale).toBe(LOCALE_STORAGE_KEY);
    expect(tokens.STORAGE_KEY.viewState).toBe(VIEW_STATE_STORAGE_KEY);
  });

  it('never names two concepts with the same value inside one group', () => {
    for (const [group, values] of Object.entries(tokens)) {
      const list = Object.values(values as Record<string, string>);
      expect(new Set(list).size, group).toBe(list.length);
    }
  });

  it('keeps dotted method names lowercase and without spaces', () => {
    for (const method of Object.values(tokens.RPC)) expect(method).toMatch(/^[a-z][a-z0-9-]*(\.[a-z0-9-]+)+$/);
  });
});
