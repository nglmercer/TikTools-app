import { describe, expect, it } from 'vitest';
import { screenDirection } from '@/app/model/screen-motion.ts';

describe('screenDirection', () => {
  it('reads moving down the sidebar as forward', () => {
    expect(screenDirection('feed', 'points')).toBe('forward');
    expect(screenDirection('connect', 'settings')).toBe('forward');
  });

  it('reads moving up the sidebar as back', () => {
    expect(screenDirection('settings', 'feed')).toBe('back');
    expect(screenDirection('behavior', 'connect')).toBe('back');
  });

  it('puts plugin pages after the built-in tabs', () => {
    expect(screenDirection('settings', 'plugin:acme.tts:main')).toBe('forward');
    expect(screenDirection('plugin:acme.tts:main', 'feed')).toBe('back');
  });

  it('defaults to forward for the first screen and for the same position', () => {
    expect(screenDirection(null, 'feed')).toBe('forward');
    expect(screenDirection('feed', 'feed')).toBe('forward');
    expect(screenDirection('plugin:a.b:one', 'plugin:c.d:two')).toBe('forward');
  });
});
