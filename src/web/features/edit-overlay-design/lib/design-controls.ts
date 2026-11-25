import type {
  WidgetAlign,
  WidgetEditorControl,
  WidgetStyle,
  WidgetTemplateSchema,
} from '../../../../widgets/sdk/template.ts';

export interface DesignDefaults {
  background: string;
  textColor: string;
  accent: string;
  radius: number;
  borderColor: string;
  borderWidth: number;
  opacity: number;
  padding: number;
  gap: number;
  width: number;
  avatarSize: number;
  badgeSize: number;
  motionPreset: string;
  motionEnterMs: number;
  motionExitMs: number;
  motionEasing: string;
  motionStaggerMs: number;
  badgeWeight: number;
  badgeSpacing: number;
}

export function designDefaults(schema: WidgetTemplateSchema): DesignDefaults {
  const base = schema.defaultDesign;
  return {
    background: base.background ?? '#16161d',
    textColor: base.textColor ?? '#f5f5f7',
    accent: base.accent ?? '#22c55e',
    radius: base.radius ?? 16,
    borderColor: base.borderColor ?? '#2a2a33',
    borderWidth: base.borderWidth ?? 1,
    opacity: base.opacity ?? 100,
    padding: base.padding ?? 16,
    gap: base.gap ?? 16,
    width: base.width ?? 440,
    avatarSize: base.avatar?.size ?? 56,
    badgeSize: base.badge?.fontSize ?? 11,
    badgeWeight: base.badge?.fontWeight ?? 700,
    badgeSpacing: base.badge?.letterSpacing ?? 1.5,
    motionPreset: base.motion?.preset ?? 'slide-up',
    motionEnterMs: base.motion?.enterMs ?? 400,
    motionExitMs: base.motion?.exitMs ?? 400,
    motionEasing: base.motion?.easing ?? 'spring',
    motionStaggerMs: base.motion?.staggerMs ?? 35,
  };
}

type Nested = 'avatar' | 'badge' | 'motion';
type NestedValue = string | number | boolean | undefined;

export function withNested<K extends Nested>(
  design: WidgetStyle,
  group: K,
  key: keyof NonNullable<WidgetStyle[K]>,
  value: NestedValue,
): WidgetStyle {
  const current = { ...(design[group] ?? {}) } as Record<string, NestedValue>;
  if (value === undefined) delete current[key as string];
  else current[key as string] = value;
  const next = { ...design };
  if (Object.keys(current).length === 0) delete next[group];
  else (next as Record<string, unknown>)[group] = current;
  return next;
}

export function withValue<K extends keyof WidgetStyle>(design: WidgetStyle, key: K, value: WidgetStyle[K] | undefined): WidgetStyle {
  const next = { ...design };
  if (value === undefined) delete next[key];
  else next[key] = value;
  return next;
}

interface ControlBase<T> {
  label: string;
  resetKeys: readonly string[];
  get: (design: WidgetStyle, defaults: DesignDefaults) => T;
  set: (design: WidgetStyle, value: T, defaults: DesignDefaults) => WidgetStyle;
}

export interface ColorControl extends ControlBase<string> {
  kind: 'color';
}

export interface RangeControl extends ControlBase<number> {
  kind: 'range';
  min: number;
  max: number;
  step: number;
  unit: 'px' | '%' | 'ms' | '';
  disabled?: (design: WidgetStyle) => boolean;
}

export interface ToggleControl extends ControlBase<boolean> {
  kind: 'toggle';
}

export interface AlignControl extends ControlBase<WidgetAlign> {
  kind: 'align';
}

export interface ChoiceControl extends ControlBase<string> {
  kind: 'choice';
  options: readonly string[];
  labelKey: 'motionPreset' | 'motionEasing';
}

export type ScalarControl = ColorControl | RangeControl | ToggleControl | AlignControl | ChoiceControl;

export type StructuralControl = Extract<WidgetEditorControl, 'layers' | 'textFields' | 'addTextField'>;
export type ScalarControlId = Exclude<WidgetEditorControl, StructuralControl>;

function color(label: string, key: 'textColor' | 'background' | 'accent' | 'borderColor', fallback: keyof DesignDefaults): ColorControl {
  return {
    kind: 'color',
    label,
    resetKeys: [key],
    get: (design, defaults) => design[key] ?? String(defaults[fallback]),
    set: (design, value) => withValue(design, key, value),
  };
}

function range(
  label: string,
  key: 'radius' | 'borderWidth' | 'opacity' | 'padding' | 'gap' | 'width',
  bounds: Pick<RangeControl, 'min' | 'max' | 'step' | 'unit'>,
  fallback: keyof DesignDefaults,
): RangeControl {
  return {
    kind: 'range',
    label,
    resetKeys: [key],
    ...bounds,
    get: (design, defaults) => design[key] ?? Number(defaults[fallback]),
    set: (design, value) => withValue(design, key, value),
  };
}

function nestedRange<K extends Nested>(
  label: string,
  group: K,
  key: keyof NonNullable<WidgetStyle[K]>,
  bounds: Pick<RangeControl, 'min' | 'max' | 'step' | 'unit'>,
  fallback: (design: WidgetStyle, defaults: DesignDefaults) => number,
): RangeControl {
  return {
    kind: 'range',
    label,
    resetKeys: [`${group}.${String(key)}`],
    ...bounds,
    get: (design, defaults) => (design[group]?.[key as never] as number | undefined) ?? fallback(design, defaults),
    set: (design, value) => withNested(design, group, key, value),
  };
}

function nestedVisible(label: string, group: Exclude<Nested, 'motion'>): ToggleControl {
  return {
    kind: 'toggle',
    label,
    resetKeys: [`${group}.visible`],
    get: (design) => design[group]?.visible !== false,
    set: (design, value) => withNested(design, group, 'visible', value ? undefined : false),
  };
}


function motionRange(
  label: string,
  key: 'enterMs' | 'exitMs' | 'staggerMs',
  bounds: Pick<RangeControl, 'min' | 'max' | 'step'>,
  fallback: keyof DesignDefaults,
): RangeControl {
  return {
    kind: 'range',
    label,
    resetKeys: [`motion.${key}`],
    ...bounds,
    unit: 'ms',
    get: (design, defaults) => design.motion?.[key] ?? Number(defaults[fallback]),
    set: (design, value) => withNested(design, 'motion', key, value),
  };
}

function motionChoice(
  label: string,
  key: 'preset' | 'easing',
  labelKey: ChoiceControl['labelKey'],
  options: readonly string[],
  fallback: keyof DesignDefaults,
): ChoiceControl {
  return {
    kind: 'choice',
    label,
    labelKey,
    options,
    resetKeys: [`motion.${key}`],
    get: (design, defaults) => design.motion?.[key] ?? String(defaults[fallback]),
    set: (design, value) => withNested(design, 'motion', key, value),
  };
}

export type ControlLabels = Record<ScalarControlId, string>;

export function scalarControls(labels: ControlLabels): Record<ScalarControlId, ScalarControl> {
  return {
    textColor: color(labels.textColor, 'textColor', 'textColor'),
    background: color(labels.background, 'background', 'background'),
    accent: color(labels.accent, 'accent', 'accent'),
    borderColor: color(labels.borderColor, 'borderColor', 'borderColor'),
    radius: range(labels.radius, 'radius', { min: 0, max: 48, step: 1, unit: 'px' }, 'radius'),
    borderWidth: range(labels.borderWidth, 'borderWidth', { min: 0, max: 8, step: 1, unit: 'px' }, 'borderWidth'),
    opacity: range(labels.opacity, 'opacity', { min: 0, max: 100, step: 1, unit: '%' }, 'opacity'),
    padding: range(labels.padding, 'padding', { min: 0, max: 64, step: 1, unit: 'px' }, 'padding'),
    gap: range(labels.gap, 'gap', { min: 0, max: 48, step: 1, unit: 'px' }, 'gap'),
    width: {
      ...range(labels.width, 'width', { min: 240, max: 720, step: 4, unit: 'px' }, 'width'),
      disabled: (design) => design.width === undefined,
    },
    shadow: {
      kind: 'toggle',
      label: labels.shadow,
      resetKeys: ['shadow'],
      get: (design) => design.shadow !== false,
      set: (design, value) => withValue(design, 'shadow', value ? undefined : false),
    },
    autoWidth: {
      kind: 'toggle',
      label: labels.autoWidth,
      resetKeys: ['width'],
      get: (design) => design.width === undefined,
      set: (design, value, defaults) => withValue(design, 'width', value ? undefined : defaults.width),
    },
    align: {
      kind: 'align',
      label: labels.align,
      resetKeys: ['align'],
      get: (design) => design.align ?? 'left',
      set: (design, value) => withValue(design, 'align', value === 'left' ? undefined : value),
    },
    avatarVisible: nestedVisible(labels.avatarVisible, 'avatar'),
    avatarSize: nestedRange(labels.avatarSize, 'avatar', 'size', { min: 16, max: 160, step: 1, unit: 'px' }, (_, d) => d.avatarSize),
    avatarRadius: nestedRange(labels.avatarRadius, 'avatar', 'radius', { min: 0, max: 80, step: 1, unit: 'px' }, (design, d) =>
      Math.min(80, (design.avatar?.size ?? d.avatarSize) / 2),
    ),
    avatarBorderColor: {
      kind: 'color',
      label: labels.avatarBorderColor,
      resetKeys: ['avatar.borderColor'],
      get: (design, defaults) => design.avatar?.borderColor ?? defaults.accent,
      set: (design, value) => withNested(design, 'avatar', 'borderColor', value),
    },
    avatarBorderWidth: nestedRange(labels.avatarBorderWidth, 'avatar', 'borderWidth', { min: 0, max: 8, step: 1, unit: 'px' }, () => 0),
    headingVisible: nestedVisible(labels.headingVisible, 'badge'),
    headingColor: {
      kind: 'color',
      label: labels.headingColor,
      resetKeys: ['badge.color'],
      get: (design, defaults) => design.badge?.color ?? defaults.accent,
      set: (design, value) => withNested(design, 'badge', 'color', value),
    },
    headingFontSize: nestedRange(labels.headingFontSize, 'badge', 'fontSize', { min: 8, max: 32, step: 1, unit: 'px' }, (_, d) => d.badgeSize),
    headingWeight: nestedRange(labels.headingWeight, 'badge', 'fontWeight', { min: 400, max: 900, step: 100, unit: '' }, (_, d) => d.badgeWeight),
    headingSpacing: nestedRange(labels.headingSpacing, 'badge', 'letterSpacing', { min: 0, max: 8, step: 0.5, unit: 'px' }, (_, d) => d.badgeSpacing),
    motionPreset: motionChoice(labels.motionPreset, 'preset', 'motionPreset', ['slide-up', 'slide-down', 'slide-left', 'slide-right', 'pop', 'fade', 'none'], 'motionPreset'),
    motionEnterMs: motionRange(labels.motionEnterMs, 'enterMs', { min: 0, max: 2000, step: 50 }, 'motionEnterMs'),
    motionExitMs: motionRange(labels.motionExitMs, 'exitMs', { min: 0, max: 2000, step: 50 }, 'motionExitMs'),
    motionEasing: motionChoice(labels.motionEasing, 'easing', 'motionEasing', ['spring', 'standard', 'linear'], 'motionEasing'),
    motionStaggerMs: motionRange(labels.motionStaggerMs, 'staggerMs', { min: 0, max: 500, step: 5 }, 'motionStaggerMs'),
  };
}

export function isStructuralControl(control: WidgetEditorControl): control is StructuralControl {
  return control === 'layers' || control === 'textFields' || control === 'addTextField';
}

export function withoutKeys(design: WidgetStyle, keys: readonly string[]): WidgetStyle {
  let next = { ...design };
  for (const key of keys) {
    const [parent, child] = key.split('.');
    if (child && (parent === 'text' || parent === 'avatar' || parent === 'badge' || parent === 'motion')) {
      const nested = next[parent];
      if (!nested) continue;
      const updated = { ...nested } as Record<string, unknown>;
      delete updated[child];
      next = { ...next };
      if (Object.keys(updated).length > 0) (next as Record<string, unknown>)[parent] = updated;
      else delete next[parent];
    } else {
      delete next[key as keyof WidgetStyle];
    }
  }
  return next;
}
