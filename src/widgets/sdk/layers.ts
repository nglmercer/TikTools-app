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
