import { orderedTextFields, textDefaults, type TextField } from './text.ts';
import type { WidgetLayer, WidgetLayerSeed, WidgetStyle, WidgetTemplateSchema } from './template.ts';

export function defaultWidgetLayers(
  kind: keyof typeof textDefaults,
  schema: WidgetTemplateSchema,
  design: WidgetStyle,
  label: (seed: WidgetLayerSeed) => string,
): WidgetLayer[] {
  const ordered = orderedTextFields(schema.textFields, design.textOrder);
  const media = schema.defaultLayers.filter((seed) => seed.kind !== 'text');
  const text = schema.defaultLayers.filter((seed) => seed.kind === 'text')
    .sort((left, right) => ordered.indexOf(left.field as TextField) - ordered.indexOf(right.field as TextField));
  const seeds = [...media, ...text];
  const defaults = textDefaults[kind] as Partial<Record<TextField, string>>;
  return seeds.filter((seed) => {
    if (seed.kind === 'avatar' && design.avatar?.visible === false) return false;
    if (seed.field && design.hiddenText?.includes(seed.field)) return false;
    if (seed.field === 'title' && design.badge?.visible === false) return false;
    return true;
  }).map((seed) => ({
    id: seed.id,
    kind: seed.kind,
    name: label(seed),
    ...(seed.field ? { field: seed.field, text: design.text?.[seed.field] ?? defaults[seed.field] ?? '' } : {}),
  }));
}

export function updateWidgetLayer(layers: readonly WidgetLayer[], id: string, update: (layer: WidgetLayer) => WidgetLayer): WidgetLayer[] {
  return layers.map((layer) => layer.id === id ? update(layer) : layer);
}

/** A simple removal can keep the original widget geometry and styling. */
export function canRenderLegacyLayers(kind: keyof typeof textDefaults, design: WidgetStyle): boolean {
  const layers = design.layers;
  if (!layers) return true;
  const defaults = textDefaults[kind] as Partial<Record<TextField, string>>;
  const fields = orderedTextFields(Object.keys(textDefaults[kind]) as TextField[], design.textOrder)
    .filter((field) => field !== 'streakTitle');
  const expected = [...(kind === 'gift' ? ['art'] : []), 'avatar', ...fields.map((field) => `field:${field}`)];
  let previous = -1;
  for (const layer of layers) {
    const index = expected.indexOf(layer.id);
    if (index <= previous) return false;
    previous = index;
    if (layer.kind === 'text') {
      const field = layer.field;
      if (!field || layer.id !== `field:${field}` || layer.text !== (design.text?.[field] ?? defaults[field] ?? '')) return false;
      if (layer.color !== undefined || layer.fontSize !== undefined || layer.fontWeight !== undefined) return false;
    } else if (layer.id !== layer.kind || layer.size !== undefined || (layer.placement !== undefined && layer.placement !== 'left')) {
      return false;
    }
  }
  const requiredMedia = kind === 'gift' ? ['art', 'avatar'] : ['avatar'];
  return requiredMedia.every((id) => layers.some((layer) => layer.id === id));
}
