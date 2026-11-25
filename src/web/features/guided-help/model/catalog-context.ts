import type { InjectionKey } from 'vue';
import type { TourId } from './types.ts';

/** Host-provided catalog constraints/launcher, used by embedded demo workspaces. */
export interface GuideCatalogContext {
  tours?: readonly TourId[];
  onLaunch?: (id: TourId, trigger?: HTMLElement) => void;
}

export const guideCatalogContextKey: InjectionKey<GuideCatalogContext> = Symbol('guide-catalog-context');
