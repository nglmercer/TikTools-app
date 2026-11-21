import type { Component } from 'vue';
import IconChart from '@/shared/ui/icon/icons/IconChart.vue';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconCoins from '@/shared/ui/icon/icons/IconCoins.vue';
import IconGitMerge from '@/shared/ui/icon/icons/IconGitMerge.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconLink from '@/shared/ui/icon/icons/IconLink.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconRows from '@/shared/ui/icon/icons/IconRows.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
import { tourCatalogs } from '../catalogs/index.ts';
import { tourRegistry } from './registry.ts';
import type { TourId, TourLocale } from './types.ts';

/** Filter chips shared by every guided-help entry point. */
export const tutorialTags = [
  'events',
  'actions',
  'connection',
  'plugins',
  'overlays',
  'points',
  'changes',
  'diagnostics',
  'feed',
] as const;
export type TutorialTag = (typeof tutorialTags)[number];

export interface TutorialEntry {
  id: TourId;
  tags: readonly TutorialTag[];
  icon: Component;
}

export const tagIcons: Record<TutorialTag, Component> = {
  events: IconZap,
  actions: IconRows,
  connection: IconLink,
  plugins: IconPuzzle,
  overlays: IconLayers,
  points: IconCoins,
  changes: IconGitMerge,
  diagnostics: IconChart,
  feed: IconChat,
};

/** All driver.js tours, in the order they appear in the catalog. */
export const tutorialEntries: readonly TutorialEntry[] = [
  { id: 'automation', tags: ['events', 'actions'], icon: IconZap },
  { id: 'changes', tags: ['events', 'actions', 'changes'], icon: IconGitMerge },
  { id: 'connection', tags: ['connection'], icon: IconLink },
  { id: 'plugin', tags: ['plugins'], icon: IconPuzzle },
  { id: 'overlay', tags: ['overlays'], icon: IconLayers },
  { id: 'points', tags: ['points'], icon: IconCoins },
  { id: 'diagnostics', tags: ['diagnostics'], icon: IconChart },
  { id: 'feed', tags: ['feed'], icon: IconChat },
];

export interface TutorialStepText {
  id: string;
  title: string;
  description: string;
}

function catalogText(catalog: Record<string, unknown>, key: string): string {
  let value: unknown = catalog;
  for (const part of key.split('.')) {
    value = value && typeof value === 'object' ? (value as Record<string, unknown>)[part] : undefined;
  }
  return typeof value === 'string' ? value : '';
}

/** Steps are read from the same catalogs the driver.js coordinator uses. */
export function tutorialSteps(locale: TourLocale, id: TourId): readonly TutorialStepText[] {
  const catalog = tourCatalogs[locale] as unknown as Record<string, unknown>;
  return tourRegistry[id].steps.map((step) => ({
    id: step.id,
    title: catalogText(catalog, step.titleKey),
    description: catalogText(catalog, step.descriptionKey),
  }));
}
