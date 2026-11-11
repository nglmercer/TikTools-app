import { describe, expect, it } from 'vitest';
import {
  hasValidCanvas,
  overlayAspectRatio,
  overlayBoxStyle,
  overlayFrameStyle,
  overlayOpacity,
} from '../../shared/cloud-overlay-geometry.ts';

describe('cloud overlay geometry', () => {
  const canvas = { width: 1920, height: 1080 };

  it('lays elements out as percentages of the canvas', () => {
    expect(overlayBoxStyle(canvas, { x: 960, y: 108, width: 480, height: 540 })).toEqual({
      left: '50%', top: '10%', width: '25%', height: '50%', opacity: '1',
    });
  });

  it('keeps opacity within 0 to 1 and defaults to opaque', () => {
    expect(overlayOpacity(undefined)).toBe(1);
    expect(overlayOpacity(0)).toBe(0);
    expect(overlayOpacity(0.4)).toBe(0.4);
    expect(overlayOpacity(7)).toBe(1);
    expect(overlayOpacity(-1)).toBe(0);
    expect(overlayOpacity(Number.NaN)).toBe(1);
  });

  it('derives the frame aspect ratio from the canvas', () => {
    expect(overlayAspectRatio(canvas)).toBe('1920 / 1080');
    expect(overlayFrameStyle({ canvas })).toEqual({ aspectRatio: '1920 / 1080' });
  });

  it('yields no layout for a missing document or invalid canvas dimensions', () => {
    for (const invalid of [null, undefined, { width: 0, height: 1080 }, { width: 1920, height: -1 }, { width: Number.NaN, height: 10 }, { width: Infinity, height: 10 }]) {
      expect(hasValidCanvas(invalid)).toBe(false);
      expect(overlayAspectRatio(invalid)).toBeNull();
      expect(overlayBoxStyle(invalid, { x: 0, y: 0, width: 10, height: 10 })).toBeNull();
    }
    expect(overlayFrameStyle(null)).toEqual({});
    expect(overlayFrameStyle({ canvas: { width: 0, height: 0 } })).toEqual({});
  });
});
