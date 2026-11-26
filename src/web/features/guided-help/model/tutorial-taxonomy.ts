/** Product areas shown as the catalog's top-level categories. */
export const tutorialCategories = [
  'automations',
  'connection',
  'plugins',
  'overlays',
  'points',
  'diagnostics',
  'feed',
  'account',
  'analytics',
  'settings',
] as const;
/** Internal product areas can have a category for future/direct tours without appearing as public catalog sections. */
export const internalTutorialCategories = ['rankings'] as const;
export type TutorialCategory = (typeof tutorialCategories)[number] | (typeof internalTutorialCategories)[number];

/** Topic chips carried by individual guides. Events/actions/variables mirror the automation tabs. */
export const tutorialTags = [
  'events',
  'actions',
  'variables',
  'connection',
  'plugins',
  'overlays',
  'points',
  'diagnostics',
  'feed',
  'account',
  'analytics',
  'settings',
] as const;
export type TutorialTag = (typeof tutorialTags)[number];

const categoriesByScope = {
  automations: 'automations',
  connections: 'connection',
  plugins: 'plugins',
  overlays: 'overlays',
  points: 'points',
  diagnostics: 'diagnostics',
  feed: 'feed',
  account: 'account',
  analytics: 'analytics',
  settings: 'settings',
  rankings: 'rankings',
} as const satisfies Record<string, TutorialCategory>;

/** The product scope declared by the tour is the only source of its catalog category. */
export function tourCategory(module: { id: string; scope: string }): TutorialCategory {
  const category = (categoriesByScope as Record<string, TutorialCategory | undefined>)[module.scope];
  if (!category) throw new Error(`tour ${module.id}: scope ${module.scope} is not a catalog category`);
  return category;
}
