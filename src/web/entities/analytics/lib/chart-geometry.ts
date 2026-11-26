export interface ChartBox {
  width: number;
  height: number;
  pad: number;
}

export interface ChartPoint {
  x: number;
  y: number;
  value: number;
}

export interface LineGeometry {
  line: string;
  area: string;
  points: ChartPoint[];
  max: number;
}

export interface StackSegment {
  key: string;
  y: number;
  height: number;
  value: number;
}

export interface StackedBar {
  index: number;
  x: number;
  width: number;
  segments: StackSegment[];
}

function round(value: number): number {
  return Math.round(value * 100) / 100;
}

export function seriesMax(values: readonly number[]): number {
  return values.reduce((top, value) => Math.max(top, value), 0);
}

export function buildLineGeometry(values: readonly number[], box: ChartBox, explicitMax?: number): LineGeometry {
  const max = explicitMax ?? seriesMax(values);
  const innerWidth = Math.max(0, box.width - box.pad * 2);
  const innerHeight = Math.max(0, box.height - box.pad * 2);
  const base = round(box.pad + innerHeight);
  const points = values.map((value, index) => ({
    x: round(values.length <= 1 ? box.pad + innerWidth / 2 : box.pad + (innerWidth * index) / (values.length - 1)),
    y: round(max > 0 ? box.pad + innerHeight - (innerHeight * value) / max : base),
    value,
  }));
  const first = points[0];
  const last = points.at(-1);
  if (!first || !last) return { line: '', area: '', points, max };
  const line = points.map((point, index) => `${index === 0 ? 'M' : 'L'}${point.x},${point.y}`).join(' ');
  return { line, area: `${line} L${last.x},${base} L${first.x},${base} Z`, points, max };
}

export function buildStackedBars(
  layers: ReadonlyArray<{ key: string; values: readonly number[] }>,
  slots: number,
  box: ChartBox,
): { bars: StackedBar[]; max: number } {
  const totals = Array.from({ length: slots }, (_, index) =>
    layers.reduce((sum, layer) => sum + Math.max(0, layer.values[index] ?? 0), 0),
  );
  const max = seriesMax(totals);
  const innerHeight = Math.max(0, box.height - box.pad * 2);
  const slot = (box.width - box.pad * 2) / Math.max(1, slots);
  const width = Math.max(2, Math.min(18, slot * 0.62));
  const base = box.height - box.pad;
  const bars = totals.map((_, index) => {
    let offset = 0;
    const segments: StackSegment[] = [];
    for (const layer of layers) {
      const value = Math.max(0, layer.values[index] ?? 0);
      if (value <= 0 || max <= 0) continue;
      const height = Math.max(2, (innerHeight * value) / max);
      segments.push({ key: layer.key, y: round(base - offset - height), height: round(height), value });
      offset += height;
    }
    return { index, x: round(box.pad + slot * index + (slot - width) / 2), width: round(width), segments };
  });
  return { bars, max };
}
