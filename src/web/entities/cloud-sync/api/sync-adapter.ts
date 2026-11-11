import type { Component } from 'vue';
import type { JsonValue } from '../../../../shared/json.ts';

export type { JsonValue };

/** How one element differs from its synced version. */
export type SyncChangeStatus = 'modified' | 'added' | 'deleted' | 'conflict';
/** Where the difference comes from: `local` is sent, `remote` is fetched, `conflict` needs a choice. */
export type SyncChangeSide = 'local' | 'remote' | 'conflict';

/**
 * One element of a section that differs from Cloud (or from the last-known snapshot).
 *
 * `before` and `after` are the two sides the diff shows:
 * - side `local`: before = the synced (base) version, after = the local version;
 * - side `remote`: before = the local version, after = the Cloud version that would arrive;
 * - side `conflict`: before = base, after = local, `remote` = Cloud (the diff compares local with Cloud).
 * An absent side means the element does not exist there.
 */
export interface SyncChange {
  id: string;
  /** Section-defined kind, translated by `SyncAdapter.kindLabel`. */
  kind: string;
  name: string;
  status: SyncChangeStatus;
  side: SyncChangeSide;
  before?: JsonValue;
  after?: JsonValue;
  remote?: JsonValue;
  base?: JsonValue;
  /** Name of the device that made a fetched change. */
  device?: string;
}

/** What the person chose for a conflict. `merged` carries the field-level result. */
export type SyncResolution =
  | { choice: 'local' }
  | { choice: 'cloud' }
  | { choice: 'merged'; value: JsonValue };

export type SyncOperationKind = 'push' | 'fetch' | 'merge' | 'reset' | 'restore' | 'sync';

/** One recorded sync operation and the elements it changed. */
export interface SyncOperation {
  id: string;
  kind: SyncOperationKind;
  title: string;
  device?: string;
  /** Milliseconds since the Unix epoch. */
  at: number;
  changes: SyncChange[];
}

/** The panel renders a control only when the adapter declares the matching capability. */
export interface SyncCapabilities {
  /** Send the checked elements (shows checkboxes, the push icon and the send button). */
  push: boolean;
  fetch: boolean;
  /** Fetch, merge and send as one operation (the primary button). */
  sync: boolean;
  /** Put an element, or everything, back to the synced version. */
  reset: boolean;
  /** Resolve conflicts (keep local or take Cloud). */
  resolve: boolean;
  /** Choose per field when resolving a conflict. */
  merge: boolean;
  /** Force push and force pull. */
  force: boolean;
  history: boolean;
  fork: boolean;
  branches: boolean;
  autoSync: boolean;
}

export const noSyncCapabilities: Readonly<SyncCapabilities> = Object.freeze({
  push: false,
  fetch: false,
  sync: false,
  reset: false,
  resolve: false,
  merge: false,
  force: false,
  history: false,
  fork: false,
  branches: false,
  autoSync: false,
});

export interface SyncScope {
  id: string;
  label: string;
  icon?: Component;
}

export interface SyncStatusInfo {
  /** Seconds since the Unix epoch, as `sync.status` reports it. */
  lastSyncedAt?: number | null;
}

/**
 * How one section of the app takes part in sync. The panel only talks to this interface, so a
 * later Variables or Overlays adapter plugs in without touching it. Every operation is optional
 * and only called when the matching capability is true.
 */
export interface SyncAdapter {
  scope: SyncScope;
  capabilities: SyncCapabilities;
  /** Translates `SyncChange.kind`. */
  kindLabel(kind: string): string;
  list(): Promise<SyncChange[]>;
  /** A getter the panel and button watch to re-list when local data changes. */
  watchSource?(): unknown;
  /** True when the section needs a signed-in, opted-in Cloud account (default true). */
  requiresCloud?: boolean;
  /** Why the section cannot sync on this host right now (translated), or null. */
  unavailableReason?(): string | null;
  /** Short explanation shown in the status strip, e.g. what the list is compared with. */
  notice?(): string | undefined;
  status?(): SyncStatusInfo;
  push?(ids: readonly string[]): Promise<void>;
  fetch?(): Promise<void>;
  sync?(): Promise<void>;
  reset?(id: string): Promise<void>;
  resetAll?(): Promise<void>;
  resolve?(id: string, resolution: SyncResolution): Promise<void>;
  forcePush?(): Promise<void>;
  forcePull?(): Promise<void>;
  history?(): Promise<SyncOperation[]>;
  /** Restore one element of an operation, or revert the whole operation when `id` is omitted. */
  restore?(operationId: string, id?: string): Promise<void>;
  /** Reads and writes the auto-sync preference. */
  autoSync?: { get(): boolean; set(value: boolean): Promise<void> };
  /** Starts the "fork a public template" flow. */
  fork?(): Promise<void>;
}

/** Declares an adapter with every capability false unless stated. */
export function defineSyncAdapter(
  adapter: Omit<SyncAdapter, 'capabilities'> & { capabilities?: Partial<SyncCapabilities> },
): SyncAdapter {
  return { ...adapter, capabilities: { ...noSyncCapabilities, ...adapter.capabilities } };
}
