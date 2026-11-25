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

/** All driver.js tours, in the order they appear in the catalog. */
export const tutorialEntries: readonly TutorialEntry[] = [
  { id: 'automationEvents', tags: ['events'], icon: IconZap },
  { id: 'automationActions', tags: ['actions'], icon: IconRows },
  { id: 'automationVariables', tags: ['events', 'actions', 'changes'], icon: IconHash },
  { id: 'changes', tags: ['events', 'actions', 'changes'], icon: IconGitMerge },
  { id: 'automationEventChat', tags: ['events'], icon: IconChat },
  { id: 'automationEventGift', tags: ['events'], icon: IconZap },
  { id: 'automationEventSocial', tags: ['events'], icon: IconZap },
  { id: 'automationEventRoom', tags: ['events'], icon: IconZap },
  { id: 'automationActionMedia', tags: ['actions'], icon: IconRows },
  { id: 'automationActionTts', tags: ['actions'], icon: IconRows },
  { id: 'automationActionHttp', tags: ['actions'], icon: IconRows },
  { id: 'automationActionVars', tags: ['actions', 'changes'], icon: IconHash },
  { id: 'automationFilters', tags: ['events'], icon: IconZap },
  { id: 'automationCooldowns', tags: ['events'], icon: IconZap },
  { id: 'automationTemplates', tags: ['events', 'actions'], icon: IconRows },
  { id: 'connection', tags: ['connection'], icon: IconLink },
  { id: 'connectionRecovery', tags: ['connection'], icon: IconLink },
  { id: 'plugin', tags: ['plugins'], icon: IconPuzzle },
  { id: 'pluginSetup', tags: ['plugins'], icon: IconPuzzle },
  { id: 'overlay', tags: ['overlays'], icon: IconLayers },
  { id: 'overlayObs', tags: ['overlays'], icon: IconLayers },
  { id: 'cloudOverlays', tags: ['overlays', 'account'], icon: IconCloud },
  { id: 'points', tags: ['points'], icon: IconCoins },
  { id: 'rankings', tags: ['points', 'rankings'], icon: IconCoins },
  { id: 'analytics', tags: ['analytics'], icon: IconChart },
  { id: 'diagnostics', tags: ['diagnostics'], icon: IconChart },
  { id: 'feed', tags: ['feed'], icon: IconChat },
  { id: 'feedInspector', tags: ['feed'], icon: IconChat },
  { id: 'settings', tags: ['settings'], icon: IconGear },
  { id: 'backup', tags: ['account'], icon: IconCloud },
  { id: 'backupRestore', tags: ['account'], icon: IconCloud },
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
