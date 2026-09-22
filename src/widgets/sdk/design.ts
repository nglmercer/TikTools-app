import type { WidgetAlign, WidgetAvatarStyle, WidgetBadgeStyle, WidgetStyle } from './template.ts';
import { textFields, type TextField } from './text.ts';

const COLOR_PATTERN = /^#[\da-f]{6}([\da-f]{2})?$/i;

function readColor(source: Record<string, unknown>, key: string): string | undefined {
  const value = source[key];
  return typeof value === 'string' && COLOR_PATTERN.test(value) ? value : undefined;
}

function readNumber(source: Record<string, unknown>, key: string, min: number, max: number): number | undefined {
  const value = source[key];
  if (typeof value !== 'number' || !Number.isFinite(value)) return undefined;
  return Math.max(min, Math.min(max, value));
}

function readBoolean(source: Record<string, unknown>, key: string): boolean | undefined {
  const value = source[key];
  return typeof value === 'boolean' ? value : undefined;
}

function normalizeAvatar(value: unknown): WidgetAvatarStyle | undefined {
  if (!value || typeof value !== 'object') return undefined;
  const source = value as Record<string, unknown>;
  const result: WidgetAvatarStyle = {};
  const visible = readBoolean(source, 'visible');
  if (visible !== undefined) result.visible = visible;
  const size = readNumber(source, 'size', 16, 160);
  if (size !== undefined) result.size = size;
  const radius = readNumber(source, 'radius', 0, 80);
  if (radius !== undefined) result.radius = radius;
  const borderColor = readColor(source, 'borderColor');
  if (borderColor !== undefined) result.borderColor = borderColor;
  const borderWidth = readNumber(source, 'borderWidth', 0, 8);
  if (borderWidth !== undefined) result.borderWidth = borderWidth;
  return Object.keys(result).length > 0 ? result : undefined;
}

function normalizeBadge(value: unknown): WidgetBadgeStyle | undefined {
  if (!value || typeof value !== 'object') return undefined;
  const source = value as Record<string, unknown>;
  const result: WidgetBadgeStyle = {};
  const visible = readBoolean(source, 'visible');
  if (visible !== undefined) result.visible = visible;
  const color = readColor(source, 'color');
  if (color !== undefined) result.color = color;
  const fontSize = readNumber(source, 'fontSize', 8, 32);
  if (fontSize !== undefined) result.fontSize = fontSize;
  const weight = readNumber(source, 'fontWeight', 100, 900);
  if (weight !== undefined) result.fontWeight = Math.max(400, Math.min(900, Math.round(weight / 100) * 100));
  const letterSpacing = readNumber(source, 'letterSpacing', 0, 8);
  if (letterSpacing !== undefined) result.letterSpacing = letterSpacing;
  return Object.keys(result).length > 0 ? result : undefined;
}

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
  if (Array.isArray(source.hiddenText)) {
    const hidden = [...new Set(source.hiddenText.filter((field): field is TextField =>
      typeof field === 'string' && (textFields as string[]).includes(field),
    ))].slice(0, textFields.length);
    if (hidden.length > 0) result.hiddenText = hidden;
  }
  for (const key of ['background', 'textColor', 'accent', 'borderColor'] as const) {
    const color = readColor(source, key);
    if (color !== undefined) result[key] = color;
  }
  const radius = readNumber(source, 'radius', 0, 48);
  if (radius !== undefined) result.radius = radius;
  const borderWidth = readNumber(source, 'borderWidth', 0, 8);
  if (borderWidth !== undefined) result.borderWidth = borderWidth;
  const shadow = readBoolean(source, 'shadow');
  if (shadow !== undefined) result.shadow = shadow;
  const opacity = readNumber(source, 'opacity', 0, 100);
  if (opacity !== undefined) result.opacity = opacity;
  const padding = readNumber(source, 'padding', 0, 64);
  if (padding !== undefined) result.padding = padding;
  const gap = readNumber(source, 'gap', 0, 48);
  if (gap !== undefined) result.gap = gap;
  const align = source.align;
  if (align === 'left' || align === 'center' || align === 'right') {
    result.align = align as WidgetAlign;
  }
  const width = readNumber(source, 'width', 240, 720);
  if (width !== undefined) result.width = width;
  const avatar = normalizeAvatar(source.avatar);
  if (avatar !== undefined) result.avatar = avatar;
  const badge = normalizeBadge(source.badge);
  if (badge !== undefined) result.badge = badge;
  return result;
}

export function parseDesign(json: string): WidgetStyle {
  try { return normalizeDesign(JSON.parse(json)); } catch { return {}; }
}

export function designFromHash(hash: string): WidgetStyle {
  return parseDesign(new URLSearchParams(hash.replace(/^#/, '')).get('design') ?? '{}');
}
