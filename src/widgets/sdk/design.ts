import type { WidgetStyle } from './template.ts';
import { textFields } from './text.ts';

/** Only portable, validated style tokens may cross the editor/OBS boundary. */
export function normalizeDesign(value: unknown): WidgetStyle {
  const result: WidgetStyle = {};
  if (!value || typeof value !== 'object') return result;
  const source = value as Record<string, unknown>;
  if (source.text && typeof source.text === 'object') {
    const fields = source.text as Record<string, unknown>;
    result.text = {};
    for (const key of textFields) {
      if (typeof fields[key] === 'string') result.text[key] = fields[key].slice(0, 300);
    }
  }
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
