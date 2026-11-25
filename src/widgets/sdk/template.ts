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

/** Avatar area styling. All fields optional; unset falls back to CSS. */
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

/** Heading line styling (e.g. "NEW FOLLOWER"). The stored badge key remains
 * for compatibility with existing widget design URLs. */
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

export type WidgetMotionPreset = 'none' | 'fade' | 'slide-up' | 'slide-down' | 'slide-left' | 'slide-right' | 'pop';
export type WidgetMotionEasing = 'linear' | 'standard' | 'spring';

/** Portable motion preferences shared by the editor, preview and OBS widget. */
export interface WidgetMotionStyle {
  preset?: WidgetMotionPreset;
  /** Enter transition duration in ms. */
  enterMs?: number;
  /** Exit transition duration in ms. */
  exitMs?: number;
  easing?: WidgetMotionEasing;
  /** Small per-row delay used by list widgets, in ms. */
  staggerMs?: number;
}

export type WidgetTemplateToken = 'name' | 'username' | 'gift' | 'count' | 'diamonds' | 'message';

export interface WidgetLayer {
  id: string;
  kind: 'text' | 'avatar' | 'art';
  name: string;
  text?: string;
  /** Original event field, used only for legacy visibility and default styling. */
  field?: TextField;
  color?: string;
  fontSize?: number;
  fontWeight?: number;
  size?: number;
  /** Media placement around the text in its row; card position moves the group. */
  placement?: 'left' | 'right' | 'top' | 'bottom';
}

export type WidgetLayerSeed = { id: string; kind: WidgetLayer['kind']; field?: TextField };

/** Built-in controls that a widget template may place in an editor section. */
export type WidgetEditorControl =
  | 'layers' | 'textFields' | 'addTextField' | 'textColor'
  | 'background' | 'accent' | 'radius' | 'borderColor' | 'borderWidth' | 'shadow' | 'opacity'
  | 'align' | 'autoWidth' | 'width' | 'padding' | 'gap'
  | 'avatarVisible' | 'avatarSize' | 'avatarRadius' | 'avatarBorderColor' | 'avatarBorderWidth'
  | 'headingVisible' | 'headingColor' | 'headingFontSize' | 'headingWeight' | 'headingSpacing'
  | 'motionPreset' | 'motionEnterMs' | 'motionExitMs' | 'motionEasing' | 'motionStaggerMs';

export interface WidgetEditorGroup {
  title?: 'fields' | 'textAppearance' | 'appearance' | 'layout' | 'motion';
  controls: readonly WidgetEditorControl[];
}

/** Section and control ownership is declared by the template. Heading fields
 * belong to the rendered kicker line; hiding all hides that section. */
export type WidgetEditorSection =
  | { id: 'layers' | 'text' | 'card' | 'image'; groups: readonly WidgetEditorGroup[] }
  | { id: 'heading'; textFields: readonly TextField[]; groups: readonly WidgetEditorGroup[] };

/** JSON-serializable styling contract for an editor; no HTML or scripts. */
export interface WidgetStyle {
  /** Ordered content layers. When set, these replace the legacy fixed lines. */
  layers?: WidgetLayer[];
  text?: Partial<Record<import('./text.ts').TextField, string>>;
  /** Template fields to hide from the rendered alert. Absent = render all. */
  hiddenText?: import('./text.ts').TextField[];
  /** Top-to-bottom order of text lines. Missing fields follow template order. */
  textOrder?: import('./text.ts').TextField[];
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
  motion?: WidgetMotionStyle;
}

/**
 * The portable contract shared by the desktop editor, the local preview and
 * the standalone OBS bundle. A template owns its supported fields and its
 * visual defaults; consumers must not recreate those values per surface.
 */
export interface WidgetTemplateSchema {
  defaultLayers: readonly WidgetLayerSeed[];
  textFields: readonly TextField[];
  /** Fields the editor keeps available without a remove control. */
  requiredTextFields: readonly TextField[];
  tokens: readonly WidgetTemplateToken[];
  editorSections: readonly WidgetEditorSection[];
  defaultDesign: WidgetStyle;
}

/** The editor and OBS instantiate the same template, with independent state. */
export interface WidgetTemplate {
  id: string;
  schema: WidgetTemplateSchema;
  topics?: readonly string[];
  resetOnDisconnect?: boolean;
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
const MOTION_PRESETS: Record<WidgetMotionPreset, { enter: string; exit: string }> = {
  none: { enter: 'none', exit: 'none' },
  fade: { enter: 'none', exit: 'none' },
  'slide-up': { enter: 'translateY(18px) scale(0.98)', exit: 'translateY(10px) scale(0.98)' },
  'slide-down': { enter: 'translateY(-18px) scale(0.98)', exit: 'translateY(-10px) scale(0.98)' },
  'slide-left': { enter: 'translateX(24px) scale(0.98)', exit: 'translateX(16px) scale(0.98)' },
  'slide-right': { enter: 'translateX(-24px) scale(0.98)', exit: 'translateX(-16px) scale(0.98)' },
  pop: { enter: 'scale(0.88)', exit: 'scale(1.04)' },
};
const MOTION_EASING: Record<WidgetMotionEasing, { enter: string; exit: string }> = {
  linear: { enter: 'linear', exit: 'linear' },
  standard: { enter: 'cubic-bezier(0.2, 0.9, 0.25, 1)', exit: 'cubic-bezier(0.4, 0, 0.6, 1)' },
  spring: { enter: 'cubic-bezier(0.2, 0.9, 0.25, 1.2)', exit: 'cubic-bezier(0.5, 0, 0.75, 0.4)' },
};

function msVariable(variables: Record<string, string>, name: string, value: unknown, min: number, max: number): void {
  if (typeof value === 'number' && Number.isFinite(value)) {
    variables[name] = `${Math.max(min, Math.min(max, Math.round(value)))}ms`;
  }
}

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
  const motion = style.motion;
  if (motion && typeof motion === 'object') {
    const preset = motion.preset && Object.hasOwn(MOTION_PRESETS, motion.preset) ? motion.preset : undefined;
    if (preset) {
      variables['--widget-motion-enter-transform'] = MOTION_PRESETS[preset].enter;
      variables['--widget-motion-exit-transform'] = MOTION_PRESETS[preset].exit;
      if (preset === 'none') {
        variables['--widget-motion-enter-duration'] = '0ms';
        variables['--widget-motion-exit-duration'] = '0ms';
      }
    }
    if (preset !== 'none') {
      msVariable(variables, '--widget-motion-enter-duration', motion.enterMs, 0, 2000);
      msVariable(variables, '--widget-motion-exit-duration', motion.exitMs, 0, 2000);
    }
    const easing = motion.easing && Object.hasOwn(MOTION_EASING, motion.easing) ? motion.easing : undefined;
    if (easing) {
      variables['--widget-motion-enter-easing'] = MOTION_EASING[easing].enter;
      variables['--widget-motion-exit-easing'] = MOTION_EASING[easing].exit;
    }
    msVariable(variables, '--widget-motion-stagger', motion.staggerMs, 0, 500);
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
    motion: defaults.motion || design.motion
      ? { ...(defaults.motion ?? {}), ...(design.motion ?? {}) }
      : undefined,
  };
}
