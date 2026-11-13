import { defineSyncAdapter, type JsonValue, type SyncAdapter, type SyncChange, type SyncResolution } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { loadWorkspaceEntries, workspaceJson, type WorkspaceEntry, type WorkspaceValue } from '@/entities/cloud-sync/api/workspace-changes.ts';
import { jsonEqual } from '@/shared/ui/json/json-diff.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';
import type { PluginStatus } from '../../../../automation/behavior/types.ts';

const SELECTIONS = 'plugin-selections';
const SETTINGS = 'plugin-settings';

export interface PluginSyncDeps {
  client: ControlClient;
  plugins: () => readonly PluginStatus[];
  /** `sync.status` as the sync store holds it. */
  status: () => { lastSyncedAt?: number | null; pending?: number } | null;
  refreshStatus: () => Promise<void>;
  /** `undefined` while the host's methods are still unknown. */
  supports: (method: string) => boolean | undefined;
  name: (plugin: PluginStatus) => string;
  t: (key: string) => string;
}

interface ConflictReview {
  handle: string;
  collection: string;
  key: string;
  base: WorkspaceValue;
  local: WorkspaceValue;
  cloud: WorkspaceValue;
}

const idOf = (collection: string, key: string): string => `${collection}/${key}`;

function statusOf(base: WorkspaceValue, local: WorkspaceValue): SyncChange['status'] {
  if (local.state !== 'present') return 'deleted';
  return base.state === 'present' ? 'modified' : 'added';
}

/**
 * Plugin selections (enabled, version) and the plugin settings staged for Cloud.
 *
 * Both sides come from `sync.v2.changes.list`, a local read of the v2 workspace: `before` is the
 * last Cloud value the store fetched (the base of a pending edit), `after` the staged local edit.
 * A Cloud selection that differs from the installed plugin is listed as something to fetch. Only this section is published; other downloaded collections remain cached. There is
 * no per-record send, fetch-only or field merge.
 *
 * Plugin settings are different: their values never reach the page. A settings change is listed
 * by name and the keys that changed, with no diff and no merge; a conflict can only be resolved
 * by keeping the local version or taking Cloud's (both run on the host).
 */
export function createPluginSyncAdapter(deps: PluginSyncDeps): SyncAdapter {
  const supported = (): boolean => deps.supports('sync.v2.changes.list') !== false && deps.supports('sync.v2.section') !== false;

  const pluginName = (key: string): string => {
    const plugin = deps.plugins().find((entry) => entry.descriptor.id === key);
    return plugin ? deps.name(plugin) : key;
  };

  function localSelection(key: string): JsonValue | undefined {
    const plugin = deps.plugins().find((entry) => entry.descriptor.id === key);
    if (!plugin?.installed) return undefined;
    return { pluginId: key, version: plugin.descriptor.version, enabled: plugin.enabled };
  }

  function change(entry: WorkspaceEntry): SyncChange | null {
    const kind = entry.collection === SELECTIONS ? 'plugin' : 'pluginSettings';
    const id = idOf(entry.collection, entry.key);
    const name = pluginName(entry.key);
    const pending = entry.pending;
    if (entry.collection === SETTINGS) {
      // Settings can hold anything, so no value is read or shown: only which keys changed. The host
      // already omits the values; this stays true even if an older host still sent them.
      if (!pending) return null;
      const changedKeys = [...(pending.changedKeys ?? [])];
      if (pending.status !== 'queued') return { id, kind, name, status: 'conflict', side: 'conflict', changedKeys };
      return { id, kind, name, status: statusOf(pending.base, pending.local), side: 'local', changedKeys };
    }
    if (pending && pending.status !== 'queued') {
      return {
        id, kind, name, status: 'conflict', side: 'conflict',
        base: workspaceJson(pending.base), before: workspaceJson(pending.base), after: workspaceJson(pending.local), remote: workspaceJson(entry.remote),
      };
    }
    if (pending) {
      return { id, kind, name, status: statusOf(pending.base, pending.local), side: 'local', before: workspaceJson(pending.base), after: workspaceJson(pending.local) };
    }
    if (entry.collection !== SELECTIONS || entry.remote.state !== 'present') return null;
    const remote = workspaceJson(entry.remote);
    const local = localSelection(entry.key);
    if (remote === undefined) return null;
    const comparable = (value: JsonValue | undefined): unknown => {
      const object = value as { version?: unknown; enabled?: unknown } | undefined;
      return object ? { version: object.version, enabled: object.enabled } : undefined;
    };
    if (local === undefined && (remote as { enabled?: unknown }).enabled !== true) return null;
    if (jsonEqual(comparable(local), comparable(remote))) return null;
    return { id, kind, name, status: local === undefined ? 'added' : 'modified', side: 'remote', before: local, after: remote };
  }

  async function list(): Promise<SyncChange[]> {
    if (!supported()) return [];
    if (await deps.client.call<boolean>('sync.v2.selected', {}) !== true) throw new Error(deps.t('syncChanges.v2Off'));
    const entries = await loadWorkspaceEntries(deps.client);
    return entries
      .filter((entry) => entry.collection === SELECTIONS || entry.collection === SETTINGS)
      .map(change)
      .filter((item): item is SyncChange => item !== null);
  }

  async function resolve(id: string, resolution: SyncResolution): Promise<void> {
    if (resolution.choice === 'merged') throw new Error(deps.t('syncChanges.actionFailed'));
    const split = id.indexOf('/');
    const review = await deps.client.call<ConflictReview>('sync.v2.conflict.review', { collection: id.slice(0, split), key: id.slice(split + 1) });
    await deps.client.call('sync.v2.conflict.resolve', { handle: review.handle, choice: resolution.choice === 'local' ? 'keepLocal' : 'takeCloud' });
    await deps.refreshStatus();
  }

  return defineSyncAdapter({
    scope: { id: 'plugins', label: deps.t('syncChanges.scopes.plugins') },
    capabilities: { sync: true, resolve: true },
    kindLabel: (kind) => deps.t(`syncChanges.kinds.${kind}`),
    unavailableReason: () => (supported() ? null : deps.t('syncChanges.unsupported')),
    watchSource: () => [deps.plugins().map((plugin) => `${plugin.descriptor.id}:${plugin.installed}:${plugin.enabled}:${plugin.descriptor.version}`).join('|'), deps.status()?.lastSyncedAt, deps.status()?.pending],
    notice: () => deps.t('syncChanges.pluginsNotice'),
    status: () => ({ lastSyncedAt: deps.status()?.lastSyncedAt ?? null }),
    list,
    async sync() {
      if (!supported()) throw new Error(deps.t('syncChanges.unsupported'));
      await deps.client.call('sync.v2.section', { section: 'plugins' });
      await deps.refreshStatus();
    },
    resolve,
  });
}
