import { describe, expect, it } from 'vitest';
import { shallowRef } from 'vue';
import { createFeedViewport } from '@/entities/live-event/model/feed-viewport.ts';
import * as tokens from '../../shared/index.ts';
import { LOCALE_STORAGE_KEY } from '@/shared/i18n/locale-preference.ts';
import { VIEW_STATE_STORAGE_KEY } from '@/shared/lib/view-state.ts';
import { isRecord } from '../support/json.ts';

describe('shared test tokens', () => {
  it('pins the storage keys to the ones the app writes', () => {
    expect(tokens.STORAGE_KEY.locale).toBe(LOCALE_STORAGE_KEY);
    expect(tokens.STORAGE_KEY.viewState).toBe(VIEW_STATE_STORAGE_KEY);
  });

  it('pins the feed render limit to the viewport default', () => {
    expect(createFeedViewport(shallowRef([]), () => true).renderLimit.value).toBe(tokens.LIMIT.feedRenderLimit);
  });

  it('never names two concepts with the same value inside one group', () => {
    for (const [group, values] of Object.entries(tokens)) {
      if (!isRecord(values)) throw new Error(`Expected token group ${group} to be an object`);
      const list = Object.values(values);
      expect(new Set(list).size, group).toBe(list.length);
    }
  });

  it('keeps dotted method names lowercase and without spaces', () => {
    for (const method of Object.values(tokens.RPC)) expect(method).toMatch(/^[a-z][a-z0-9-]*(\.[a-z0-9-]+)+$/);
  });
});
