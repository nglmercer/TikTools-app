import type { AppTab } from '@/types.ts';
import { builtinTabPaths } from '../router/tab-routes.ts';

export type ScreenDirection = 'forward' | 'back';

/** Sidebar order: the built-in tabs as listed in the navigation, then every plugin page after them. */
const builtinOrder: readonly string[] = Object.keys(builtinTabPaths);

const positionOf = (tab: AppTab | null): number => {
  if (tab === null) return builtinOrder.length;
  const index = builtinOrder.indexOf(tab);
  return index >= 0 ? index : builtinOrder.length;
};

/**
 * Which way a screen change reads. Moving down the sidebar is forward, moving up is back; two plugin pages (or
 * a first screen with no previous one) read as forward. The screen transition uses it to enter
 * from below or from above, so the motion matches the list the user just clicked in.
 */
export function screenDirection(from: AppTab | null, to: AppTab | null): ScreenDirection {
  if (from === null) return 'forward';
  return positionOf(to) < positionOf(from) ? 'back' : 'forward';
}
