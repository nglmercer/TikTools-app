import { afterEach, describe, expect, test } from 'bun:test';

import {
  resetAutocompleteRegistry,
  syncAutocompletePluginStates,
} from '../autocomplete-registry.ts';
import { fieldsForTrigger, findField, operatorsFor, operatorsForPath } from './fields.ts';
import { matchesFilter } from './filters.ts';
import { sampleEventFor } from './samples.ts';

describe('condition fields', () => {
  test('every field a trigger offers resolves against that trigger sample event', () => {
    const sample = sampleEventFor('tiktok.gift');
    const paths = fieldsForTrigger('tiktok.gift').map((field) => field.path);

    expect(paths).toContain('event.data.giftName');
    expect(paths).toContain('event.user.uniqueId');
    // The gift sample carries every gift field, so an `eq` against its own
    // value must pass — that is what proves the paths are not typos.
    expect(matchesFilter({ path: 'event.data.giftName', operator: 'eq', value: 'Rosa' }, sample)).toBe(true);
    expect(matchesFilter({ path: 'event.data.diamondCount', operator: 'gte', value: '1' }, sample)).toBe(true);
  });

  test('the operators on offer match the kind of value the field holds', () => {
    expect(operatorsFor('number')).toContain('gte');
    expect(operatorsFor('number')).not.toContain('contains');
    expect(operatorsFor('boolean')).toEqual(['is-true', 'is-false']);
    expect(operatorsFor('gift')).toEqual(['eq', 'neq', 'in']);
    expect(operatorsFor('text')).toContain('starts-with');
  });

  test('a gift field asks for the gift picker and a viewer field for the viewer picker', () => {
    expect(findField('tiktok.gift', 'event.data.giftName')?.kind).toBe('gift');
    expect(findField('tiktok.gift', 'event.user.uniqueId')?.kind).toBe('user');
    expect(findField('tiktok.gift', 'event.data.repeatEnd')?.kind).toBe('boolean');
  });

  test('a hand-written path is unknown, which is what makes the editor fall back to free text', () => {
    expect(findField('tiktok.gift', 'event.data.whatever')).toBeUndefined();
  });

  test('every trigger offers at least one field', () => {
    for (const trigger of ['tiktok.gift', 'tiktok.chat', 'tiktok.like', 'points.awarded', 'tiktok.follow'] as const) {
      expect(fieldsForTrigger(trigger).length).toBeGreaterThan(0);
    }
  });
});

describe('operatorsForPath', () => {
  test('registry paths keep their kind operators', () => {
    expect(findField('tiktok.chat', 'event.data.comment')?.kind).toBe('text');
    expect(operatorsForPath('tiktok.chat', 'event.data.comment')).toEqual([
      'eq',
      'neq',
      'in',
      'contains',
      'starts-with',
    ]);
    expect(findField('tiktok.gift', 'event.data.repeatEnd')?.kind).toBe('boolean');
    expect(operatorsForPath('tiktok.gift', 'event.data.repeatEnd')).toEqual([
      'is-true',
      'is-false',
    ]);
  });

  test('custom paths offer text operators plus boolean assertions', () => {
    // Provider-namespaced enrichment is deliberately absent from the
    // registry; the advanced custom path still needs `is-true` so the
    // moderation `blocked` verdict stays filterable and editable.
    const path = 'event.intel.providers.textintel.comment.moderation.blocked';
    expect(findField('tiktok.chat', path)).toBeUndefined();
    expect(operatorsForPath('tiktok.chat', path)).toEqual([
      'eq',
      'neq',
      'in',
      'contains',
      'starts-with',
      'is-true',
      'is-false',
    ]);
  });
});

describe('plugin availability gating', () => {
  afterEach(() => {
    resetAutocompleteRegistry();
  });

  test('condition fields hide textintel enrichment while it is disabled', () => {
    syncAutocompletePluginStates([{ id: 'textintel', installed: true, enabled: false }]);
    const paths = fieldsForTrigger('tiktok.chat').map((field) => field.path);
    expect(paths.some((path) => path.startsWith('event.intel.comment.'))).toBe(false);
    expect(paths.some((path) => path.startsWith('event.intel.user.'))).toBe(false);
    // Core data fields and the host-stamped processing status survive.
    expect(paths).toContain('event.data.comment');
    expect(paths).toContain('event.intel.processing.status');
  });

  test('condition fields offer textintel enrichment while it is available', () => {
    const paths = fieldsForTrigger('tiktok.chat').map((field) => field.path);
    expect(paths).toContain('event.intel.comment.tts.text');
    syncAutocompletePluginStates([{ id: 'textintel', installed: true, enabled: true }]);
    expect(fieldsForTrigger('tiktok.chat').map((field) => field.path))
      .toContain('event.intel.comment.tts.text');
  });
});
