/**
 * The registry is assembled from the tour modules in ../tours/. Entries are
 * derived by the shape factories in define-tour.ts; do not edit by hand.
 */
import { mergeCatalog, registryEntry, tourLocales, type TourCopyLocale } from './define-tour.ts';
import { tourModules, type TourId } from '../tours/index.ts';
import type { TourDefinition } from './types.ts';

export type { TourId };

export const tourRegistry = Object.fromEntries(
  tourModules.map((module) => [module.id, registryEntry(module)]),
) as Record<TourId, TourDefinition>;

type StepText = { title: string; description: string };
type ControlText = { next: string; previous: string; done: string; close: string; progress: string };

/** Catalog shape mapped from the literal tour modules, so consumers get
 *  concrete deep access (tourCatalogs.en.automationEvents.eventsTab.title)
 *  without any per-tour type declarations. */
export type TourCatalogShape = {
  [M in (typeof tourModules)[number] as M['id']]: {
    [S in keyof M['steps']]: StepText;
  };
} & {
  controls: ControlText;
  card: { [M in (typeof tourModules)[number] as M['id']]: StepText };
};

export const tourCatalogs = Object.fromEntries(
  tourLocales.map((locale: TourCopyLocale) => [locale, mergeCatalog(tourModules, locale)]),
) as unknown as Record<TourCopyLocale, TourCatalogShape>;

export function isTourId(value: string): value is TourId {
  return Object.hasOwn(tourRegistry, value);
}
