import { beforeEach, describe, expect, it } from 'vitest';

import {
  clearEmoteImageFailures,
  emoteImageFailureCount,
  hasEmoteImageFailed,
  markEmoteImageFailed,
} from '../../../../../../src/web/entities/live-event/lib/emote-failures.ts';

beforeEach(() => clearEmoteImageFailures());

describe('emote image failure cache', () => {
  it('remembers a failed URL so it is never requested twice', () => {
    const url = 'https://cdn.example.com/a.webp';
    expect(hasEmoteImageFailed(url)).toBe(false);
    markEmoteImageFailed(url);
    expect(hasEmoteImageFailed(url)).toBe(true);
  });

  it('is idempotent', () => {
    markEmoteImageFailed('https://cdn.example.com/a.webp');
    markEmoteImageFailed('https://cdn.example.com/a.webp');
    expect(emoteImageFailureCount()).toBe(1);
  });

  it('evicts oldest entries so the set stays bounded', () => {
    for (let index = 0; index < 300; index += 1) {
      markEmoteImageFailed(`https://cdn.example.com/${index}.webp`);
    }
    expect(emoteImageFailureCount()).toBeLessThanOrEqual(256);
    // The newest entries survive; the oldest were evicted.
    expect(hasEmoteImageFailed('https://cdn.example.com/299.webp')).toBe(true);
    expect(hasEmoteImageFailed('https://cdn.example.com/0.webp')).toBe(false);
  });

  it('clear drops every remembered failure', () => {
    markEmoteImageFailed('https://cdn.example.com/a.webp');
    clearEmoteImageFailures();
    expect(emoteImageFailureCount()).toBe(0);
    expect(hasEmoteImageFailed('https://cdn.example.com/a.webp')).toBe(false);
  });
});
