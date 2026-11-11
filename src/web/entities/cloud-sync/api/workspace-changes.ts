import type { ControlClient } from '@/shared/api/control-client.ts';
import type { JsonValue } from './sync-adapter.ts';

/** A Cloud-side or local value as `sync.v2.*` projects it. */
export interface WorkspaceValue {
  state: 'absent' | 'deleted' | 'present';
  revision: number | null;
  value: JsonValue | null;
  version: string | null;
  enabled: boolean | null;
}

export interface WorkspacePendingChange {
  status: 'queued' | 'unconfirmed' | 'conflict';
  base: WorkspaceValue;
  local: WorkspaceValue;
}

/** One durable v2 workspace entry: the last fetched Cloud value and the pending local edit. */
export interface WorkspaceEntry {
  collection: string;
  key: string;
  remote: WorkspaceValue;
  pending: WorkspacePendingChange | null;
  hasNewer: boolean;
}

/** Reads `sync.v2.changes.list`, a local-only read that never contacts Cloud. */
export async function loadWorkspaceEntries(client: ControlClient): Promise<WorkspaceEntry[]> {
  const result = await client.call<WorkspaceEntry[]>('sync.v2.changes.list', {});
  return Array.isArray(result) ? result : [];
}

/** The portable JSON of a value, or `undefined` when the record is absent or deleted. */
export function workspaceJson(value: WorkspaceValue | undefined): JsonValue | undefined {
  return value && value.state === 'present' && value.value !== null ? value.value : undefined;
}
