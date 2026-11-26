import type { Component } from 'vue';
import IconChart from '@/shared/ui/icon/icons/IconChart.vue';
import IconChat from '@/shared/ui/icon/icons/IconChat.vue';
import IconClock from '@/shared/ui/icon/icons/IconClock.vue';
import IconCloud from '@/shared/ui/icon/icons/IconCloud.vue';
import IconCode from '@/shared/ui/icon/icons/IconCode.vue';
import IconCopy from '@/shared/ui/icon/icons/IconCopy.vue';
import IconCoins from '@/shared/ui/icon/icons/IconCoins.vue';
import IconDownload from '@/shared/ui/icon/icons/IconDownload.vue';
import IconEye from '@/shared/ui/icon/icons/IconEye.vue';
import IconFilter from '@/shared/ui/icon/icons/IconFilter.vue';
import IconFlow from '@/shared/ui/icon/icons/IconFlow.vue';
import IconGear from '@/shared/ui/icon/icons/IconGear.vue';
import IconGift from '@/shared/ui/icon/icons/IconGift.vue';
import IconGlobe from '@/shared/ui/icon/icons/IconGlobe.vue';
import IconGitMerge from '@/shared/ui/icon/icons/IconGitMerge.vue';
import IconHeart from '@/shared/ui/icon/icons/IconHeart.vue';
import IconHistory from '@/shared/ui/icon/icons/IconHistory.vue';
import IconHash from '@/shared/ui/icon/icons/IconHash.vue';
import IconLayers from '@/shared/ui/icon/icons/IconLayers.vue';
import IconLink from '@/shared/ui/icon/icons/IconLink.vue';
import IconPlay from '@/shared/ui/icon/icons/IconPlay.vue';
import IconRefresh from '@/shared/ui/icon/icons/IconRefresh.vue';
import IconShield from '@/shared/ui/icon/icons/IconShield.vue';
import IconTrophy from '@/shared/ui/icon/icons/IconTrophy.vue';
import IconUsers from '@/shared/ui/icon/icons/IconUsers.vue';
import IconVideo from '@/shared/ui/icon/icons/IconVideo.vue';
import IconVolume from '@/shared/ui/icon/icons/IconVolume.vue';
import IconPuzzle from '@/shared/ui/icon/icons/IconPuzzle.vue';
import IconRows from '@/shared/ui/icon/icons/IconRows.vue';
import IconZap from '@/shared/ui/icon/icons/IconZap.vue';
import { isTourId, tourCatalogs, tourRegistry } from './registry.ts';
import { tourCategory, type TutorialCategory, type TutorialTag } from './tutorial-taxonomy.ts';
import { tourModules } from '../tours/index.ts';
import type { TourDefinition, TourId, TourLocale } from './types.ts';

export { tutorialCategories, tutorialTags, tourCategory } from './tutorial-taxonomy.ts';
export type { TutorialCategory, TutorialTag } from './tutorial-taxonomy.ts';
export interface TutorialEntry {
  id: string;
  category: TutorialCategory;
  tags: readonly TutorialTag[];
  icon: Component;
  overview: boolean;
}

export const categoryIcons: Record<TutorialCategory, Component> = {
  automations: IconFlow,
  connection: IconLink,
  plugins: IconPuzzle,
  overlays: IconLayers,
  points: IconCoins,
  diagnostics: IconShield,
  feed: IconChat,
  account: IconCloud,
  analytics: IconChart,
  settings: IconGear,
  rankings: IconTrophy,
};

export const tagIcons: Record<TutorialTag, Component> = {
  events: IconZap,
  actions: IconPlay,
  variables: IconHash,
  connection: IconLink,
  plugins: IconPuzzle,
  overlays: IconLayers,
  points: IconCoins,
  diagnostics: IconShield,
  feed: IconChat,
  account: IconCloud,
  analytics: IconChart,
  settings: IconGear,
};

const iconByName: Record<string, Component> = {
  chart: IconChart,
  chat: IconChat,
  clock: IconClock,
  cloud: IconCloud,
  code: IconCode,
  copy: IconCopy,
  coins: IconCoins,
  download: IconDownload,
  eye: IconEye,
  filter: IconFilter,
  gear: IconGear,
  gift: IconGift,
  globe: IconGlobe,
  'git-merge': IconGitMerge,
  heart: IconHeart,
  history: IconHistory,
  hash: IconHash,
  layers: IconLayers,
  link: IconLink,
  play: IconPlay,
  puzzle: IconPuzzle,
  refresh: IconRefresh,
  rows: IconRows,
  shield: IconShield,
  trophy: IconTrophy,
  users: IconUsers,
  video: IconVideo,
  volume: IconVolume,
  zap: IconZap,
};

export function tutorialIcon(name: string | undefined): Component {
  return (name ? iconByName[name] : undefined) ?? IconZap;
}

const overviewTutorialIds = new Set<string>([
  'automationEvents',
  'automationActions',
  'automationVariables',
  'connection',
  'plugin',
  'overlay',
  'points',
  'diagnostics',
  'feed',
  'backup',
  'analytics',
  'settings',
]);


/** All driver.js tours, in the order they appear in the catalog — derived from the tour modules. */
export const tutorialEntries: readonly TutorialEntry[] = tourModules
  .filter((module) => !('lifecycle' in module) || module.lifecycle !== 'internal')
  .map((module) => ({
    id: module.id as TourId,
    category: tourCategory(module),
    tags: module.tags as readonly TutorialTag[],
    icon: tutorialIcon(module.icon),
    overview: overviewTutorialIds.has(module.id),
  }));

export interface TutorialStepText {
  id: string;
  title: string;
  description: string;
}

/** Catalog entry after its card copy and step copy have been resolved for one locale. */
export interface LocalizedTutorialEntry extends TutorialEntry {
  title: string;
  description: string;
  steps: readonly TutorialStepText[];
  /** False when the host workspace lacks the example records declared by the tour contract. Undefined means the host did not provide enough state to evaluate them. */
  requirementsMet?: boolean;
  /** Runtime definition for plugin-contributed guides; static tours resolve theirs from the registry. */
  definition?: TourDefinition;
  /** Opaque dynamic launcher payload typed in the dialog to avoid coupling this catalog model to the coordinator. */
  dynamicTour?: unknown;
}

function catalogText(catalog: Record<string, unknown>, key: string): string {
  let value: unknown = catalog;
  for (const part of key.split('.')) {
    value = value && typeof value === 'object' ? (value as Record<string, unknown>)[part] : undefined;
  }
  return typeof value === 'string' ? value : '';
}

/** Steps are read from the same catalogs the driver.js coordinator uses. */
export function tutorialSteps(locale: TourLocale, id: string): readonly TutorialStepText[] {
  if (!isTourId(id)) return [];
  const catalog = tourCatalogs[locale] as unknown as Record<string, unknown>;
  return tourRegistry[id].steps.map((step) => ({
    id: step.id,
    title: catalogText(catalog, step.titleKey),
    description: catalogText(catalog, step.descriptionKey),
  }));
}

/** Card title/description colocated in the tour module, exposed through the merged catalog. */
export function tutorialCard(locale: TourLocale, id: string): { title: string; description: string } {
  if (!isTourId(id)) return { title: '', description: '' };
  const catalog = tourCatalogs[locale] as unknown as { card?: Record<string, { title: string; description: string }> };
  return catalog.card?.[id] ?? { title: '', description: '' };
}
