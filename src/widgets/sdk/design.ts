import type { WidgetStyle } from './template.ts';

/** Only portable, validated style tokens may cross the editor/OBS boundary. */
export function normalizeDesign(value: unknown): WidgetStyle {
  const result: WidgetStyle = {};
  if (!value || typeof value !== 'object') return result;
  const source = value as Record<string, unknown>;
  for (const key of ['background', 'textColor', 'accent'] as const) {
    const color = source[key];
    if (typeof color === 'string' && /^#[\da-f]{6}([\da-f]{2})?$/i.test(color)) result[key] = color;
  }
  if (typeof source.radius === 'number' && Number.isFinite(source.radius)) {
    result.radius = Math.max(0, Math.min(48, source.radius));
  }
  return result;
}

export function parseDesign(json: string): WidgetStyle {
  try { return normalizeDesign(JSON.parse(json)); } catch { return {}; }
}

export function designFromHash(hash: string): WidgetStyle {
  return parseDesign(new URLSearchParams(hash.replace(/^#/, '')).get('design') ?? '{}');
}
