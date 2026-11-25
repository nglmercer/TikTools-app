import type { Component } from 'vue';
import IconChart from '@/shared/ui/icon/icons/IconChart.vue';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconCloud from '@/shared/ui/icon/icons/IconCloud.vue';
import IconCoins from '@/shared/ui/icon/icons/IconCoins.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconGitMerge from '@/shared/ui/icon/icons/IconGitMerge.vue';
import IconHash from '@/shared/ui/icon/icons/IconHash.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconLink from '@/shared/ui/icon/icons/IconLink.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconRows from '@/shared/ui/icon/icons/IconRows.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
import { tourCatalogs } from './registry.ts';
import { tourRegistry } from './registry.ts';
import { tourModules } from '../tours/index.ts';
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
  'account',
  'analytics',
  'settings',
  'rankings',
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
  account: IconCloud,
  analytics: IconChart,
  settings: IconGear,
  rankings: IconCoins,
};

const iconByName: Record<string, Component> = {
  chart: IconChart,
  chat: IconChat,
  cloud: IconCloud,
  coins: IconCoins,
  gear: IconGear,
  'git-merge': IconGitMerge,
  hash: IconHash,
  layers: IconLayers,
  link: IconLink,
  puzzle: IconPuzzle,
  rows: IconRows,
  zap: IconZap,
};

/** All driver.js tours, in the order they appear in the catalog — derived from the tour modules. */
export const tutorialEntries: readonly TutorialEntry[] = tourModules.map((module) => ({
  id: module.id as TourId,
  tags: module.tags as readonly TutorialTag[],
  icon: iconByName[module.icon] ?? IconZap,
}));

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

/** Card title/description colocated in the tour module, exposed through the merged catalog. */
export function tutorialCard(locale: TourLocale, id: TourId): { title: string; description: string } {
  const catalog = tourCatalogs[locale] as unknown as { card?: Record<string, { title: string; description: string }> };
  return catalog.card?.[id] ?? { title: '', description: '' };
}
