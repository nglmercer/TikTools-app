/** Product areas shown as the catalog's top-level categories. */
export const tutorialCategories = [
  'automations',
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
export type TutorialCategory = (typeof tutorialCategories)[number];

/** Topic chips carried by individual guides. Events/actions/variables mirror the automation tabs. */
export const tutorialTags = [
  'events',
  'actions',
  'variables',
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

/** Automation guides form one product-area category; their events/actions/variables tags remain tab metadata. */
export function tourCategory(module: { id: string; tags: readonly string[] }): TutorialCategory {
  if (module.id.startsWith('automation')) return 'automations';
  const firstTag = module.tags[0] ?? '';
  if ((tutorialCategories as readonly string[]).includes(firstTag)) return firstTag as TutorialCategory;
  throw new Error(`tour ${module.id}: first tag ${firstTag} is not a catalog category`);
}
