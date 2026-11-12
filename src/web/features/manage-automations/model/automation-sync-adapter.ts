import { defineSyncAdapter, type JsonValue, type SyncAdapter, type SyncChange } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { advanceSnapshot, readSnapshot, snapshotRevision, writeSnapshot, type SectionSnapshot } from '@/entities/cloud-sync/lib/snapshot-store.ts';
import { jsonEqual } from '@/shared/ui/json/json-diff.ts';
import type { LiveAction, LiveEvent } from '../../../../automation/behavior/types.ts';
import type { AutomationHost, AutomationRecordVersion, AutomationRevision, AutomationWriteBase } from '@/entities/automation/model/record-versions.ts';

type AutoRecord = LiveEvent | LiveAction;
export type AutomationSyncKind = 'event' | 'action';

export interface AutomationSyncDeps {
  kind: AutomationSyncKind;
  /** The records of this kind in the active library. */
  records: () => readonly AutoRecord[];
  /** False until the host has delivered its definitions; an empty list then is not a deletion. */
  loaded: () => boolean;
  read: (kind: AutomationSyncKind, id: string) => Promise<AutomationRecordVersion>;
  save: (record: AutoRecord, base: AutomationWriteBase) => Promise<unknown>;
  remove: (kind: AutomationSyncKind, id: string, revision: AutomationRevision, host?: AutomationHost | null) => Promise<void>;
  t: (key: string) => string;
  /**
   * Where the reference copy lives; defaults to this browser's storage. `advance` replaces it on
   * purpose (mark as reviewed) and tells readers to compare again; `write` only seeds it.
   */
  snapshots?: {
    read: (scope: string) => SectionSnapshot | null;
    write: (scope: string, snapshot: SectionSnapshot) => void;
    advance?: (scope: string, snapshot: SectionSnapshot) => void;
  };
}

const toJson = (value: unknown): JsonValue => JSON.parse(JSON.stringify(value)) as JsonValue;

/**
 * Events or actions of the active automation library.
 *
 * No Cloud collection carries individual automation records yet (rule profiles are not staged
 * into the v2 workspace), so there is no Cloud base to compare with. Local records are compared
 * with the reference copy this device kept: taken the first time the section is seen, and only
 * advanced when the person marks the section as reviewed (a transport will advance it later).
 * Nothing is sent. Reset and mark as reviewed are offered.
 */
export function createAutomationSyncAdapter(deps: AutomationSyncDeps): SyncAdapter {
  const scopeId = deps.kind === 'event' ? 'events' : 'actions';
  const snapshots = deps.snapshots ?? { read: readSnapshot, write: writeSnapshot, advance: advanceSnapshot };
  const storeKey = `automation-${scopeId}`;

  const current = (): Map<string, JsonValue> => new Map(deps.records().map((record) => [record.id, toJson(record)]));
  const names = (): Map<string, string> => new Map(deps.records().map((record) => [record.id, record.name]));

  async function list(): Promise<SyncChange[]> {
    if (!deps.loaded()) return [];
    const now = current();
    const base = snapshots.read(storeKey);
    if (base === null) {
      snapshots.write(storeKey, Object.fromEntries(now));
      return [];
    }
    const labels = names();
    const out: SyncChange[] = [];
    for (const [id, value] of now) {
      const before = base[id];
      if (before === undefined) out.push({ id, kind: deps.kind, name: labels.get(id) ?? id, status: 'added', side: 'local', after: value });
      else if (!jsonEqual(before, value)) out.push({ id, kind: deps.kind, name: labels.get(id) ?? id, status: 'modified', side: 'local', before, after: value });
    }
    for (const [id, before] of Object.entries(base)) {
      if (!now.has(id)) out.push({ id, kind: deps.kind, name: (before as { name?: string }).name ?? id, status: 'deleted', side: 'local', before });
    }
    return out;
  }

  async function reset(id: string): Promise<void> {
    const baseline = snapshots.read(storeKey)?.[id];
    const version = await deps.read(deps.kind, id);
    if (baseline === undefined) {
      // Added since the snapshot: putting it back means removing it.
      if (version.record) await deps.remove(deps.kind, id, version.revision, version.host);
      return;
    }
    await deps.save(baseline as unknown as AutoRecord, { revision: version.revision, create: version.record === null, host: version.host });
  }

  return defineSyncAdapter({
    scope: { id: scopeId, label: deps.t(`syncChanges.scopes.${scopeId}`) },
    capabilities: { reset: true, markReviewed: true },
    requiresCloud: false,
    kindLabel: (kind) => deps.t(`syncChanges.kinds.${kind}`),
    watchSource: () => [deps.records(), snapshotRevision()],
    notice: () => deps.t('syncChanges.automationsNotice'),
    list,
    reset,
    async markReviewed() {
      if (!deps.loaded()) throw new Error(deps.t('syncChanges.actionFailed'));
      const next = Object.fromEntries(current());
      if (snapshots.advance) snapshots.advance(storeKey, next);
      else snapshots.write(storeKey, next);
    },
    async resetAll() {
      for (const change of await list()) await reset(change.id);
    },
  });
}
