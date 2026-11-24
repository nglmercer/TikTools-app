import { describe, expect, it } from 'vitest';
import { firstEnabledIndex, lastEnabledIndex, stepEnabledIndex, typeaheadIndex } from './list-navigation.ts';

const options = [
  { label: 'Fade', disabled: true },
  { label: 'Cut' },
  { label: 'Swipe' },
  { label: 'Slide', disabled: true },
  { label: 'Stinger' },
];

describe('list navigation', () => {
  it('skips disabled options and stays at the edges', () => {
    expect(stepEnabledIndex(options, 2, 1)).toBe(4);
    expect(stepEnabledIndex(options, 1, -1)).toBe(1);
    expect(firstEnabledIndex(options)).toBe(1);
    expect(lastEnabledIndex(options)).toBe(4);
  });

  it('cycles typeahead matches for a repeated letter', () => {
    expect(typeaheadIndex(options, 's', 1)).toBe(2);
    expect(typeaheadIndex(options, 's', 2)).toBe(4);
    expect(typeaheadIndex(options, 's', 4)).toBe(2);
  });

  it('refines the current match for longer queries', () => {
    expect(typeaheadIndex(options, 'st', 2)).toBe(4);
    expect(typeaheadIndex(options, 'zz', 0)).toBe(-1);
  });
});
