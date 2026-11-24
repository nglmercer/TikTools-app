import { describe, expect, it } from 'vitest';
import { buildLineGeometry, buildStackedBars } from '../../../../../../src/web/entities/analytics/lib/chart-geometry.ts';

const box = { width: 120, height: 60, pad: 10 };

describe('chart geometry', () => {
  it('scales a line against its maximum', () => {
    const geometry = buildLineGeometry([0, 5, 10], box);
    expect(geometry.max).toBe(10);
    expect(geometry.points.map((point) => [point.x, point.y])).toEqual([
      [10, 50],
      [60, 30],
      [110, 10],
    ]);
    expect(geometry.line).toBe('M10,50 L60,30 L110,10');
    expect(geometry.area).toBe('M10,50 L60,30 L110,10 L110,50 L10,50 Z');
  });

  it('keeps flat zero series on the baseline and handles empty input', () => {
    expect(buildLineGeometry([0, 0], box).points.every((point) => point.y === 50)).toBe(true);
    expect(buildLineGeometry([], box)).toEqual({ line: '', area: '', points: [], max: 0 });
  });

  it('stacks layers bottom-up against the tallest total', () => {
    const { bars, max } = buildStackedBars(
      [
        { key: 'a', values: [2, 0] },
        { key: 'b', values: [2, 1] },
      ],
      2,
      box,
    );
    expect(max).toBe(4);
    expect(bars[0]!.segments).toEqual([
      { key: 'a', y: 30, height: 20, value: 2 },
      { key: 'b', y: 10, height: 20, value: 2 },
    ]);
    expect(bars[1]!.segments).toEqual([{ key: 'b', y: 40, height: 10, value: 1 }]);
  });
});
