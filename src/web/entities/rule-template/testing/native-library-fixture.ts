import type { BehaviorSnapshot } from '../../../../automation/behavior/types.ts';
import { ControlCallError, type ControlCallOptions, type ResourceVersion, type RpcParams } from '@/shared/api/control-client.ts';
import type { LibraryCatalog, LibraryIntent, LibraryMutation, LibrarySnapshot, LibraryState } from '../model/library-contracts.ts';
import { libraryTarget, libraryDigest, libraryDefinitionDigest, libraryCommandHash } from '../model/library-contracts.ts';
import { scalarCompare } from '@/shared/lib/canonical-json.ts';

const copy = <T>(value: T): T => structuredClone(value);
export const fixtureDigest = libraryDigest;
export function nativeLibraryHost(business: (method: string, params?: RpcParams, options?: ControlCallOptions) => unknown,
  host: () => { hostId: string; hostInstanceId: string } = () => ({ hostId: 'host-a', hostInstanceId: 'instance-a' })) {
  let state: LibraryState = { ...host(), pending: null, recent: [] };
  let catalog: LibraryCatalog | null = null; let revision = '0'; let serial = 0; let loseReply = false;
  async function readSnapshot(): Promise<LibrarySnapshot> {
    let resourceVersions: ResourceVersion[] = [];
    const definitions = await business('automation.snapshot', {}, { onMetadata: (meta) => { resourceVersions = copy(meta?.resourceVersions ?? []); } }) as BehaviorSnapshot;
    if (catalog === null) {
      let result: { state?: Record<string, string> } | undefined;
      try { result = await business('app.state.get', { keys: ['behavior.profiles.packs', 'behavior.profiles.active', 'behavior.templates.custom'] }) as typeof result; } catch { /* old opt-in business fixtures have no catalog */ }
      catalog = { packs: JSON.parse(result?.state?.['behavior.profiles.packs'] ?? '[]'), activeId: result?.state?.['behavior.profiles.active'] ?? 'default', templates: JSON.parse(result?.state?.['behavior.templates.custom'] ?? '[]') };
    }
    resourceVersions.sort((a, b) => scalarCompare(a.scope, b.scope) || scalarCompare(a.id, b.id));
    return copy({ ...host(), revision, definitionDigest: libraryDefinitionDigest(definitions, resourceVersions), catalog, definitions, resourceVersions });
  }
  async function handle(method: string, params?: RpcParams): Promise<unknown> {
    state = { ...state, ...host() };
    if (method === 'automation.library.get' || method === 'automation.library.resolve') return copy(state);
    if (method === 'automation.library.snapshot') return readSnapshot();
    if (method === 'automation.library.review_unknown') {
      if (state.pending && state.pending.commandId === params?.commandId && params?.acknowledgeUnknown === true) {
        state.recent.unshift({ intent: state.pending, outcome: 'reviewed-unknown', receipt: null, errorCode: null, recordedAt: 2 }); state.pending = null;
      }
      return copy(state);
    }
    if (method !== 'automation.library.submit') throw new Error(`Unexpected library fixture method ${method}`);
    const before = await readSnapshot(); const mutation = copy(params?.mutation as LibraryMutation);
    const intent: LibraryIntent = { commandId: `owner-library-${(++serial).toString(16).padStart(32, '0')}`, operation: mutation.operation,
      target: libraryTarget(mutation), expectedRevision: String(params?.expectedRevision), definitionDigest: String(params?.definitionDigest),
      commandHash: libraryCommandHash(mutation, { revision: String(params?.expectedRevision), definitionDigest: String(params?.definitionDigest) }), hostInstanceId: String(params?.hostInstanceId), createdAt: 1 };
    state.pending = intent;
    const changed: ResourceVersion[] = [];
    const call = async (name: string, p: RpcParams, expected = '0') => business(name, p, { meta: { expectedRevision: expected }, onMetadata: (meta) => changed.push(...copy(meta?.resourceVersions ?? [])) });
    try {
      if (before.revision !== params?.expectedRevision || before.definitionDigest !== params?.definitionDigest) throw new ControlCallError('revision_conflict', 'Library changed');
      const current = catalog!;
      if (mutation.operation === 'create-profile') current.packs.push({ id: mutation.id, name: mutation.name, description: mutation.description, eventIds: [], actionIds: [] });
      if (mutation.operation === 'switch-profile') {
        const pack = current.packs.find((entry) => entry.id === mutation.id);
        const owned = { event: new Set(current.packs.flatMap((entry) => entry.eventIds)), action: new Set(current.packs.flatMap((entry) => entry.actionIds)) };
        const updates = [...before.definitions.events.map((record) => ({ kind: 'event', id: record.id, enabled: mutation.id === 'default' ? !owned.event.has(record.id) : !!pack?.eventIds.includes(record.id) })),
          ...before.definitions.actions.map((record) => ({ kind: 'action', id: record.id, enabled: mutation.id === 'default' ? !owned.action.has(record.id) : !!pack?.actionIds.includes(record.id) }))];
        await call('automation.set_enablement', { updates }); current.activeId = mutation.id;
      }
      if (mutation.operation === 'apply-template' || mutation.operation === 'import-profile') {
        const entries = mutation.operation === 'apply-template' ? [mutation.entry] : mutation.entries;
        for (const entry of entries) {
          for (const record of entry.actions) await call('automation.create', { kind: 'action', record });
          await call('automation.create', { kind: 'event', record: entry.event });
        }
        if (mutation.operation === 'import-profile') {
          current.packs = [...current.packs.filter((pack) => pack.id !== mutation.profile.id), { ...mutation.profile, eventIds: entries.map((entry) => entry.event.id), actionIds: entries.flatMap((entry) => entry.actions.map((record) => record.id)) }]; current.activeId = mutation.profile.id;
        } else {
          const pack = current.packs.find((pack) => pack.id === mutation.profileId);
          if (pack) { pack.eventIds.push(...entries.map((entry) => entry.event.id)); pack.actionIds.push(...entries.flatMap((entry) => entry.actions.map((record) => record.id))); }
        }
      }
      if (mutation.operation === 'delete-profile') {
        const pack = current.packs.find((pack) => pack.id === mutation.id);
        for (const [kind, ids] of [['event', pack?.eventIds ?? []], ['action', pack?.actionIds ?? []]] as const) for (const id of ids) {
          const version = before.resourceVersions.find((v) => v.scope === kind && v.id === id)?.revision ?? '0'; await call('automation.delete', { kind, id }, version);
        }
        current.packs = current.packs.filter((pack) => pack.id !== mutation.id); if (current.activeId === mutation.id) current.activeId = 'default';
      }
      if (mutation.operation === 'assign-record') {
        const key = mutation.kind === 'event' ? 'eventIds' : 'actionIds';
        current.packs = current.packs.map((pack) => ({ ...pack, [key]: [...pack[key].filter((id) => id !== mutation.id), ...(pack.id === mutation.profileId ? [mutation.id] : [])] }));
      }
      if (mutation.operation === 'import-templates') for (const template of mutation.templates) current.templates = [...current.templates.filter((entry) => entry.id !== template.id), copy(template)];
      if (mutation.operation === 'delete-template') current.templates = current.templates.filter((entry) => entry.id !== mutation.id);
    } catch (failure) {
      const code = failure instanceof ControlCallError ? failure.code : 'invalid_params';
      state.recent.unshift({ intent, outcome: 'rejected', receipt: null, errorCode: code, recordedAt: 2 }); state.pending = null;
      return { state: copy(state), acknowledgement: null };
    }
    revision = String(BigInt(revision) + 1n);
    const after = await readSnapshot();
    const versions = [{ kind: 'automation', scope: 'library', id: 'catalog', revision } as ResourceVersion,
      ...[...new Map(changed.map((version) => [`${version.scope}:${version.id}`, version])).values()].sort((a, b) => scalarCompare(a.scope, b.scope) || scalarCompare(a.id, b.id))];
    const receipt = { revision, catalogDigest: fixtureDigest(catalog), definitionDigest: after.definitionDigest, versionsDigest: fixtureDigest(versions), affectedResources: versions.length };
    state.recent.unshift({ intent, outcome: 'succeeded', receipt, errorCode: null, recordedAt: 2 }); state.recent = state.recent.slice(0, 16); state.pending = null;
    if (loseReply) { loseReply = false; throw new ControlCallError('timeout', 'Library reply lost after commit'); }
    return { state: copy(state), acknowledgement: { value: receipt, resourceVersions: versions, hostInstanceId: host().hostInstanceId,
      command: { commandId: intent.commandId, state: 'succeeded', resource: { kind: 'automation', scope: 'library', id: 'catalog' }, committedAt: 2 } } };
  }
  return { handle, readSnapshot, state: () => state, setState: (next: LibraryState) => { state = copy(next); }, setCatalog: (next: LibraryCatalog) => { catalog = copy(next); }, loseNextReply: () => { loseReply = true; } };
}
