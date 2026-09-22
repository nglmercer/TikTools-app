import type { VNode } from 'vue';
import type { DomainEventEnvelope } from '../shared/event-types.ts';
import type { GatewayStatus } from '../shared/gateway-client.ts';
import type { WidgetClock } from '../shared/clock.ts';
import type { TextField } from './text.ts';

export interface WidgetController {
  handleEnvelope(envelope: DomainEventEnvelope): unknown;
  clear(): void;
  dispose(): void;
}

/** Avatar/badge area styling. All fields optional; unset falls back to CSS. */
export interface WidgetAvatarStyle {
  visible?: boolean;
  /** Diameter in px. */
  size?: number;
  /** Corner radius in px. */
  radius?: number;
  borderColor?: string;
  /** Border width in px. */
  borderWidth?: number;
}

/** Kicker/badge line styling (e.g. "NEW FOLLOWER"). */
export interface WidgetBadgeStyle {
  visible?: boolean;
  color?: string;
  /** Font size in px. */
  fontSize?: number;
  fontWeight?: number;
  /** Letter spacing in px. */
  letterSpacing?: number;
}

export type WidgetAlign = 'left' | 'center' | 'right';

export type WidgetTemplateToken = 'name' | 'username' | 'gift' | 'count' | 'diamonds' | 'message';

/** JSON-serializable styling contract for an editor; no HTML or scripts. */
export interface WidgetStyle {
  text?: Partial<Record<import('./text.ts').TextField, string>>;
  /** Template fields to hide from the rendered alert. Absent = render all. */
  hiddenText?: import('./text.ts').TextField[];
  background?: string;
  textColor?: string;
  accent?: string;
  radius?: number;
  borderColor?: string;
  /** Card border width in px. */
  borderWidth?: number;
  /** False hides the card shadow; unset keeps the default shadow. */
  shadow?: boolean;
  /** Card opacity as a 0–100 percent. */
  opacity?: number;
  /** Card padding in px. */
  padding?: number;
  /** Card gap in px. */
  gap?: number;
  align?: WidgetAlign;
  /** Card width in px; unset means auto. */
  width?: number;
  avatar?: WidgetAvatarStyle;
  badge?: WidgetBadgeStyle;
}

/**
 * The portable contract shared by the desktop editor, the local preview and
 * the standalone OBS bundle. A template owns its supported fields and its
 * visual defaults; consumers must not recreate those values per surface.
 */
export interface WidgetTemplateSchema {
  textFields: readonly TextField[];
  /** Fields the editor keeps available without a remove control. */
  requiredTextFields: readonly TextField[];
  tokens: readonly WidgetTemplateToken[];
  defaultDesign: WidgetStyle;
}

/** The editor and OBS instantiate the same template, with independent state. */
export interface WidgetTemplate {
  id: string;
  schema: WidgetTemplateSchema;
  create(search: string, clock?: WidgetClock): {
    controller: WidgetController;
    render(status: GatewayStatus, debug: boolean): VNode;
  };
  samples(): DomainEventEnvelope[];
}

export function defineWidget(template: WidgetTemplate): WidgetTemplate {
  return template;
}

export const widgetStyleFields = [
  { key: 'background', type: 'color', label: 'Background' },
  { key: 'textColor', type: 'color', label: 'Text color' },
  { key: 'accent', type: 'color', label: 'Accent' },
  { key: 'radius', type: 'number', label: 'Corner radius', min: 0, max: 48 },
] as const;

const COLOR_PATTERN = /^#[\da-f]{6}([\da-f]{2})?$/i;

function colorVariable(variables: Record<string, string>, name: string, value: unknown): void {
  if (typeof value === 'string' && COLOR_PATTERN.test(value)) variables[name] = value;
}

function pxVariable(variables: Record<string, string>, name: string, value: unknown, min: number, max: number): void {
  if (typeof value === 'number' && Number.isFinite(value)) {
    variables[name] = `${Math.max(min, Math.min(max, value))}px`;
  }
}

const ALIGN_ITEMS: Record<WidgetAlign, string> = { left: 'flex-start', center: 'center', right: 'flex-end' };

export function styleVariables(style: WidgetStyle = {}): Record<string, string> {
  const variables: Record<string, string> = {};
  colorVariable(variables, '--widget-background', style.background);
  colorVariable(variables, '--widget-textColor', style.textColor);
  colorVariable(variables, '--widget-accent', style.accent);
  pxVariable(variables, '--widget-radius', style.radius, 0, 48);
  colorVariable(variables, '--widget-borderColor', style.borderColor);
  pxVariable(variables, '--widget-borderWidth', style.borderWidth, 0, 8);
  if (style.shadow === false) variables['--widget-shadow'] = 'none';
  if (typeof style.opacity === 'number' && Number.isFinite(style.opacity)) {
    variables['--widget-opacity'] = String(Math.max(0, Math.min(100, style.opacity)) / 100);
  }
  pxVariable(variables, '--widget-padding', style.padding, 0, 64);
  pxVariable(variables, '--widget-gap', style.gap, 0, 48);
  if (style.align === 'left' || style.align === 'center' || style.align === 'right') {
    variables['--widget-align'] = style.align;
    variables['--widget-align-items'] = ALIGN_ITEMS[style.align];
  }
  pxVariable(variables, '--widget-width', style.width, 240, 720);
  const avatar = style.avatar;
  if (avatar && typeof avatar === 'object') {
    if (avatar.visible === false) variables['--widget-avatar-display'] = 'none';
    pxVariable(variables, '--widget-avatar-size', avatar.size, 16, 160);
    pxVariable(variables, '--widget-avatar-radius', avatar.radius, 0, 80);
    colorVariable(variables, '--widget-avatar-borderColor', avatar.borderColor);
    pxVariable(variables, '--widget-avatar-borderWidth', avatar.borderWidth, 0, 8);
  }
  const badge = style.badge;
  if (badge && typeof badge === 'object') {
    if (badge.visible === false) variables['--widget-badge-display'] = 'none';
    colorVariable(variables, '--widget-badge-color', badge.color);
    pxVariable(variables, '--widget-badge-size', badge.fontSize, 8, 32);
    if (typeof badge.fontWeight === 'number' && Number.isFinite(badge.fontWeight)) {
      variables['--widget-badge-weight'] = String(Math.max(400, Math.min(900, Math.round(badge.fontWeight / 100) * 100)));
    }
    pxVariable(variables, '--widget-badge-spacing', badge.letterSpacing, 0, 8);
  }
  return variables;
}

/**
 * Applies the template defaults before rendering. Nested groups are merged so
 * a saved design can override one avatar or badge property without losing the
 * other defaults. The returned object is plain data and safe to provide to
 * the renderer through Vue's reactive tree.
 */
export function resolveWidgetDesign(template: Pick<WidgetTemplate, 'schema'>, design: WidgetStyle = {}): WidgetStyle {
  const defaults = template.schema.defaultDesign;
  return {
    ...defaults,
    ...design,
    text: defaults.text || design.text
      ? { ...(defaults.text ?? {}), ...(design.text ?? {}) }
      : undefined,
    avatar: defaults.avatar || design.avatar
      ? { ...(defaults.avatar ?? {}), ...(design.avatar ?? {}) }
      : undefined,
    badge: defaults.badge || design.badge
      ? { ...(defaults.badge ?? {}), ...(design.badge ?? {}) }
      : undefined,
  };
}
