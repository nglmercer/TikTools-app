import type { InjectionKey } from 'vue';

/** Host-provided catalog constraints/launcher, used by embedded demo workspaces. */
export interface GuideCatalogContext {
  tours?: readonly string[];
  onLaunch?: (id: string, trigger?: HTMLElement) => void;
  previewUrl?: (id: string) => string;
}

export const guideCatalogContextKey: InjectionKey<GuideCatalogContext> = Symbol('guide-catalog-context');
