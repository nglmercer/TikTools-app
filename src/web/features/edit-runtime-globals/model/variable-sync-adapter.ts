import { defineSyncAdapter, type SyncChange } from '@/entities/cloud-sync/api/sync-adapter.ts';
import { isRuntimeVariable, type RuntimeVariable } from '@/entities/runtime-global/lib/runtime-variable.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';

export const VARIABLE_SYNC_METHODS = ['globals.typed.list', 'globals.typed.replace', 'sync.v2.variables.review', 'sync.v2.variables.send', 'sync.v2.variables.fetch', 'sync.v2.variables.apply', 'sync.v2.variables.withdraw'] as const;

export interface VariableComparison {
  review: string;
  name: string;
  local: RuntimeVariable | null;
  cloud: RuntimeVariable | null;
  cloudDeleted: boolean;
  localSecret: boolean;
  blockedLocal: boolean;
  pendingStatus: string | null;
  canSend: boolean;
  canApply: boolean;
  canWithdraw: boolean;
}

export interface VariableSyncDeps {
  client: ControlClient;
  supports(method: string): boolean | undefined;
  revision(): string | null;
  status(): { lastSyncedAt?: number | null; pending?: number } | null;
  refreshStatus(): Promise<void>;
  reloadLocal(): Promise<void>;
  t(key: string): string;
}

/** IDs belong to the displayed native comparison, never to a mutable latest-list cache. */
export function createVariableSyncAdapter(deps: VariableSyncDeps) {
  const supported = () => VARIABLE_SYNC_METHODS.every((method) => deps.supports(method) === true);
  const change = (row: VariableComparison): SyncChange => {
    const hidden = row.localSecret || row.blockedLocal || row.local?.type === 'secret' || row.cloud?.type === 'secret';
    const local = hidden ? undefined : row.local ?? undefined;
    const cloud = hidden ? undefined : row.cloud ?? undefined;
    if ((local && !isRuntimeVariable(local)) || (cloud && !isRuntimeVariable(cloud))) throw new Error(deps.t('syncChanges.variableNumericHost'));
    const display = (variable: RuntimeVariable | undefined) => variable?.type === 'number' && variable.numberText !== undefined
      ? { type: 'number', value: variable.numberText }
      : variable;
    const side = hidden || (row.pendingStatus && row.pendingStatus !== 'queued') || (local && (cloud || row.cloudDeleted))
      ? 'conflict' : local ? 'local' : 'remote';
    return {
      id: row.review, name: row.name, kind: 'variable', side,
      status: side === 'conflict' ? 'conflict' : row.cloudDeleted ? 'deleted' : local && cloud ? 'modified' : 'added',
      before: display(side === 'remote' ? local : cloud),
      after: display(side === 'remote' ? cloud : local),
      remote: display(cloud),
      canSend: row.canSend && !hidden,
      canApply: row.canApply && !hidden,
      canWithdraw: row.canWithdraw,
      canResolveLocal: row.canSend && !hidden,
      canResolveCloud: row.canApply && !hidden,
      valuesHidden: hidden,
      notice: hidden ? deps.t(row.localSecret ? 'syncChanges.variableSecret' : 'syncChanges.variableBlocked') : undefined,
    };
  };
  async function publish(method: string, review: string): Promise<void> {
    const result = await deps.client.call<{ conflicts: string[] }>(method, { review });
    if (result.conflicts.length) throw new Error(deps.t('syncChanges.variableConflict'));
  }
  return defineSyncAdapter({
    scope: { id: 'variables', label: deps.t('syncChanges.scopes.variables') },
    capabilities: { push: true, fetch: true, apply: true, withdraw: true, resolve: true },
    resolveLabels: { local: deps.t('syncChanges.sendSavedValue'), cloud: deps.t('syncChanges.applyCloud') },
    kindLabel: () => deps.t('syncChanges.kinds.variable'),
    watchSource: () => [deps.revision(), deps.status()?.lastSyncedAt, deps.status()?.pending],
    unavailableReason: () => supported() ? null : deps.t('syncChanges.unsupported'),
    notice: () => deps.t('syncChanges.variablesNotice'),
    status: () => ({ lastSyncedAt: deps.status()?.lastSyncedAt }),
    async list() {
      if (!supported()) return [];
      const result = await deps.client.call<{ changes: VariableComparison[] }>('sync.v2.variables.review', {});
      return result.changes.map(change);
    },
    async push(ids) {
      try { for (const review of ids) await publish('sync.v2.variables.send', review); }
      finally { await deps.refreshStatus(); }
    },
    async fetch() {
      await deps.client.call('sync.v2.variables.fetch', {});
      await deps.refreshStatus();
    },
    async apply(review) {
      try { await deps.client.call('sync.v2.variables.apply', { review }); }
      finally { await deps.reloadLocal(); await deps.refreshStatus(); }
    },
    async withdraw(review) {
      try { await publish('sync.v2.variables.withdraw', review); }
      finally { await deps.refreshStatus(); }
    },
    async resolve(review, resolution) {
      if (resolution.choice === 'merged') throw new Error(deps.t('syncChanges.actionFailed'));
      try {
        if (resolution.choice === 'local') await publish('sync.v2.variables.send', review);
        else await deps.client.call('sync.v2.variables.apply', { review });
      } finally { await deps.reloadLocal(); await deps.refreshStatus(); }
    },
  });
}
