import type { ActionTypeDefinition, PluginStatus } from '../../../../automation/behavior/types.ts';
import type { SearchEntry, UpdateInfo } from '@/entities/plugin-package/api/package-types.ts';

export type CatalogRow =
  | { key: string; kind: 'local'; plugin: PluginStatus }
  | { key: string; kind: 'remote'; entry: SearchEntry }
  | { key: string; kind: 'update'; update: UpdateInfo; plugin: PluginStatus | undefined };


export interface PluginCatalogProps {
  plugins: readonly PluginStatus[];
  actionTypes: readonly ActionTypeDefinition[];
  remote: readonly SearchEntry[];
  updates: readonly UpdateInfo[];
  searching?: boolean;
  searchError?: string;
  checkingUpdates?: boolean;
  /** False when no enabled registry exists (official one unpublished). Only the registry-based Explore reads it. */
  marketplaceAvailable?: boolean;
  /**
   * Explore lists the Cloud catalog (`catalog.items`) instead of the registry search. Hosts
   * that do not announce it keep the registry-based list.
   */
  exploreCatalog?: boolean;
  /** The package review is open: a second .plugin cannot be chosen yet. */
  installDisabled?: boolean;
}
