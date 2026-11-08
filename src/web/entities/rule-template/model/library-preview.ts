import type { LibraryMutation, LibrarySnapshot } from './library-contracts.ts';
import { resolvedPacks } from './rule-profiles-state.ts';
import type { AutomationRecord } from '@/entities/automation/model/record-versions.ts';

export interface LibraryPreviewRow { kind: 'action' | 'event'; name: string; before: AutomationRecord | null; after: AutomationRecord | null }
export function libraryPreview(context: LibrarySnapshot, mutation: LibraryMutation): LibraryPreviewRow[] {
  const records = [...context.definitions.actions.map((record) => ({ kind: 'action' as const, record })), ...context.definitions.events.map((record) => ({ kind: 'event' as const, record }))];
  const packs = resolvedPacks(context.catalog.packs, context.definitions.events.map((r) => r.id), context.definitions.actions.map((r) => r.id));
  if (mutation.operation === 'apply-template' || mutation.operation === 'import-profile') {
    const entries = mutation.operation === 'apply-template' ? [mutation.entry, ...(mutation.additionalEntries ?? [])] : mutation.entries;
    return entries.flatMap<LibraryPreviewRow>((entry) => [...entry.actions.map((record) => ({ kind: 'action' as const, name: record.name, before: null, after: record })), { kind: 'event', name: entry.event.name, before: null, after: entry.event }]);
  }
  if (mutation.operation === 'switch-profile') {
    const target = packs.find((pack) => pack.id === mutation.id);
    return records.map(({ kind, record }) => ({ kind, name: record.name, before: record,
      after: { ...record, enabled: (kind === 'action' ? target?.actionIds : target?.eventIds)?.includes(record.id) ?? false } }));
  }
  if (mutation.operation === 'delete-profile') {
    const target = packs.find((pack) => pack.id === mutation.id);
    const actions = new Set(target?.actionIds ?? []); const events = new Set(target?.eventIds ?? []);
    return records.flatMap<LibraryPreviewRow>(({ kind, record }) => {
      if ((kind === 'action' ? actions : events).has(record.id)) return [{ kind, name: record.name, before: record, after: null }];
      if (kind === 'event' && 'actionIds' in record && record.actionIds.some((id) => actions.has(id))) return [{ kind, name: record.name, before: record, after: { ...record, actionIds: record.actionIds.filter((id) => !actions.has(id)) } }];
      return [];
    });
  }
  if (mutation.operation === 'assign-record') return records.filter(({ kind, record }) => kind === mutation.kind && record.id === mutation.id).map(({ kind, record }) => ({ kind, name: record.name, before: record, after: record }));
  return [];
}

/** Compare the original frozen targets with current bodies, including absent
 * creates/deletes. A fresh plan would lose the old profile's deleted members. */
export function libraryCurrentPreview(original: LibrarySnapshot, current: LibrarySnapshot, mutation: LibraryMutation): LibraryPreviewRow[] {
  const plan = libraryPreview(original, mutation);
  const records = new Map<string, AutomationRecord>([...current.definitions.actions.map((record) => [`action:${record.id}`, record] as const), ...current.definitions.events.map((record) => [`event:${record.id}`, record] as const)]);
  const rows = plan.map((row) => {
    const id = row.before?.id ?? row.after!.id; const saved = records.get(`${row.kind}:${id}`) ?? null;
    return { ...row, name: saved?.name ?? row.name, before: saved };
  });
  if (mutation.operation === 'switch-profile') {
    const targets = new Set(plan.map((row) => `${row.kind}:${row.before!.id}`));
    for (const [key, record] of records) if (!targets.has(key)) rows.push({ kind: key.startsWith('action:') ? 'action' : 'event', name: record.name, before: record, after: null });
  }
  return rows;
}
