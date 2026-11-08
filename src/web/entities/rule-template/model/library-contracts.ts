import type { BehaviorSnapshot } from '../../../../automation/behavior/types.ts';
import { ControlCallError, type CommandReceipt, type ResourceVersion } from '@/shared/api/control-client.ts';
import { exact, object } from '@/entities/automation/model/definition-contracts.ts';
import { assertRecord, validRevision, type AutomationKind } from '@/entities/automation/model/record-versions.ts';
import { sourceNumbersSafe } from '@/shared/lib/json-source.ts';
import { parseRuleTemplate, type AppliedRuleTemplate, type RuleTemplate } from '../lib/rule-templates.ts';
import type { ProfilePack } from './rule-profiles-state.ts';
import { canonicalJson, scalarCompare } from '@/shared/lib/canonical-json.ts';
import { sha256 } from '@/shared/lib/sha256.ts';

export type LibraryMutation =
  | { operation: 'create-profile'; id: string; name: string; description: string }
  | { operation: 'switch-profile' | 'delete-profile'; id: string }
  | { operation: 'apply-template'; entry: AppliedRuleTemplate; profileId: string; additionalEntries?: AppliedRuleTemplate[]; templates?: RuleTemplate[] }
  | { operation: 'import-profile'; profile: { id: string; name: string; description: string }; entries: AppliedRuleTemplate[]; templates?: RuleTemplate[] }
  | { operation: 'assign-record'; kind: AutomationKind; id: string; profileId: string }
  | { operation: 'import-templates'; templates: RuleTemplate[] }
  | { operation: 'delete-template'; id: string };
export type LibraryOperation = LibraryMutation['operation'];
export interface LibraryCatalog { packs: ProfilePack[]; activeId: string; templates: RuleTemplate[] }
export interface LibrarySnapshot {
  hostId: string; hostInstanceId: string; revision: string; definitionDigest: string;
  catalog: LibraryCatalog; definitions: BehaviorSnapshot; resourceVersions: ResourceVersion[];
}
export interface LibraryIntent {
  commandId: string; operation: LibraryOperation; target: string; expectedRevision: string;
  definitionDigest: string; commandHash: string; hostInstanceId: string; createdAt: number;
}
export interface LibraryReceipt { revision: string; catalogDigest: string; definitionDigest: string; versionsDigest: string; affectedResources: number }
export interface LibraryRecord { intent: LibraryIntent; outcome: 'succeeded' | 'rejected' | 'reviewed-unknown'; receipt: LibraryReceipt | null; errorCode: string | null; recordedAt: number }
export interface LibraryState { hostId: string; hostInstanceId: string; pending: LibraryIntent | null; recent: LibraryRecord[] }
export interface LibraryAcknowledgement { value: LibraryReceipt; resourceVersions: ResourceVersion[]; command: CommandReceipt; hostInstanceId: string }
export interface LibrarySubmission { state: LibraryState; acknowledgement: LibraryAcknowledgement | null }
export const libraryOperations: readonly LibraryOperation[] = ['create-profile', 'switch-profile', 'delete-profile', 'apply-template', 'import-profile', 'assign-record', 'import-templates', 'delete-template'];
export const stableLibraryJson = canonicalJson;
export const libraryDigest = (value: unknown): string => sha256(canonicalJson(value));
export const libraryCommandHash = (mutation: LibraryMutation, context: Pick<LibrarySnapshot, 'revision' | 'definitionDigest'>): string => libraryDigest({
  method: 'automation.library.apply', params: { mutation, definitionDigest: context.definitionDigest }, expectedRevision: context.revision,
});
export function libraryDefinitionDigest(definitions: Pick<BehaviorSnapshot, 'actions' | 'events'>, resourceVersions: ResourceVersion[]): string {
  return libraryDigest({ actions: [...definitions.actions].sort((a, b) => scalarCompare(a.id, b.id)), events: [...definitions.events].sort((a, b) => scalarCompare(a.id, b.id)),
    resourceVersions: [...resourceVersions].sort((a, b) => scalarCompare(a.scope, b.scope) || scalarCompare(a.id, b.id)) });
}
export const sameLibraryHost = (a: { hostId: string; hostInstanceId: string }, b: { hostId: string; hostInstanceId: string }): boolean => a.hostId === b.hostId && a.hostInstanceId === b.hostInstanceId;
const digest = (v: unknown): v is string => typeof v === 'string' && /^[a-f0-9]{64}$/.test(v);
const rustTrim = (value: string) => value.replace(/^\p{White_Space}+|\p{White_Space}+$/gu, '');
const text = (v: unknown, max = 256): v is string => typeof v === 'string' && v.length > 0 && new TextEncoder().encode(v).length <= max && rustTrim(v) === v && !/\p{Cc}/u.test(v);
const id = (v: unknown): v is string => text(v, 128) && !v.includes('*');
const profileId = (v: unknown): v is string => typeof v === 'string' && !!v && Array.from(v).length <= 64 && rustTrim(v) === v && !/\p{Cc}/u.test(v);
const time = (v: unknown): v is number => typeof v === 'number' && Number.isSafeInteger(v) && v >= 0;
const invalid = (): never => { throw new ControlCallError('protocol', 'Invalid native Owner library response.'); };
const errors = ['revision_conflict', 'not_found', 'invalid_params', 'forbidden', 'capability_unavailable', 'host_restarted', 'too_large'];
const templateValue = (value: unknown): boolean => object(value) && value.templateVersion === 1 && profileId(value.id) &&
  (typeof value.title === 'string' || object(value.title) && typeof value.title.default === 'string') &&
  Array.isArray(value.actions) && value.actions.length > 0 && value.actions.length <= 8 && value.actions.every((action) => object(action) && typeof action.name === 'string' && typeof action.typeId === 'string') &&
  object(value.event) && typeof value.event.name === 'string' && typeof value.event.trigger === 'string' && parseRuleTemplate(value).ok;
export function libraryTarget(mutation: LibraryMutation): string {
  if ('id' in mutation) return mutation.id;
  if (mutation.operation === 'apply-template') return mutation.profileId;
  if (mutation.operation === 'import-profile') return mutation.profile.id;
  return 'templates';
}
function parseIntent(v: unknown): LibraryIntent {
  if (!exact(v, ['commandId', 'operation', 'target', 'expectedRevision', 'definitionDigest', 'commandHash', 'hostInstanceId', 'createdAt']) ||
    typeof v.commandId !== 'string' || !/^owner-library-[0-9a-f]{32}$/.test(v.commandId) || !libraryOperations.includes(v.operation as LibraryOperation) ||
    (v.operation === 'import-templates' ? v.target !== 'templates' : v.operation === 'assign-record' ? !id(v.target) : !profileId(v.target)) ||
    !validRevision(v.expectedRevision) || !digest(v.definitionDigest) || !digest(v.commandHash) || !text(v.hostInstanceId) || !time(v.createdAt)) return invalid();
  return v as unknown as LibraryIntent;
}
function parseReceipt(v: unknown, intent: LibraryIntent): LibraryReceipt {
  if (!exact(v, ['revision', 'catalogDigest', 'definitionDigest', 'versionsDigest', 'affectedResources']) || !validRevision(v.revision) ||
    BigInt(v.revision) !== BigInt(intent.expectedRevision) + 1n || !digest(v.catalogDigest) || !digest(v.definitionDigest) || !digest(v.versionsDigest) ||
    typeof v.affectedResources !== 'number' || !Number.isSafeInteger(v.affectedResources) || v.affectedResources < 1 || v.affectedResources > 4_294_967_295) return invalid();
  return v as unknown as LibraryReceipt;
}
export function parseLibraryState(v: unknown): LibraryState {
  if (!exact(v, ['hostId', 'hostInstanceId', 'pending', 'recent']) || !text(v.hostId) || !text(v.hostInstanceId) || !Array.isArray(v.recent) || v.recent.length > 16) return invalid();
  const pending = v.pending === null ? null : parseIntent(v.pending);
  const recent: LibraryRecord[] = v.recent.map((entry) => {
    if (!exact(entry, ['intent', 'outcome', 'receipt', 'errorCode', 'recordedAt']) || !['succeeded', 'rejected', 'reviewed-unknown'].includes(entry.outcome as string) || !time(entry.recordedAt)) return invalid();
    const intent = parseIntent(entry.intent);
    if (entry.outcome === 'succeeded') {
      if (entry.errorCode !== null) return invalid();
      return { intent, outcome: 'succeeded', receipt: parseReceipt(entry.receipt, intent), errorCode: null, recordedAt: entry.recordedAt };
    }
    if (entry.receipt !== null || (entry.outcome === 'rejected' ? !errors.includes(entry.errorCode as string) : entry.errorCode !== null)) return invalid();
    return { intent, outcome: entry.outcome as 'rejected' | 'reviewed-unknown', receipt: null, errorCode: entry.errorCode as string | null, recordedAt: entry.recordedAt };
  });
  const ids = recent.map((record) => record.intent.commandId);
  if (new Set(ids).size !== ids.length || pending && ids.includes(pending.commandId)) return invalid();
  return { hostId: v.hostId, hostInstanceId: v.hostInstanceId, pending, recent };
}
function parseVersions(v: unknown, library: boolean): ResourceVersion[] {
  if (!Array.isArray(v)) return invalid();
  const seen = new Set<string>();
  let catalog = 0;
  let previous: ResourceVersion | null = null;
  for (const [index, version] of v.entries()) {
    if (!exact(version, ['kind', 'scope', 'id', 'revision']) || version.kind !== 'automation' || !id(version.id) || !validRevision(version.revision) ||
      (library && index === 0 ? version.scope !== 'library' || version.id !== 'catalog' : !['action', 'event'].includes(version.scope as string))) return invalid();
    if (version.scope === 'library') ++catalog;
    else {
      if (previous && (scalarCompare(previous.scope, version.scope as string) || scalarCompare(previous.id, version.id)) >= 0) return invalid();
      previous = version as unknown as ResourceVersion;
    }
    const key = `${version.scope}:${version.id}`;
    if (seen.has(key)) return invalid();
    seen.add(key);
  }
  if (library && catalog !== 1) return invalid();
  return v as ResourceVersion[];
}
export function parseLibrarySnapshot(v: unknown): LibrarySnapshot {
  if (!exact(v, ['hostId', 'hostInstanceId', 'revision', 'definitionDigest', 'catalog', 'definitions', 'resourceVersions']) ||
    !text(v.hostId) || !text(v.hostInstanceId) || !validRevision(v.revision) || !digest(v.definitionDigest) ||
    !exact(v.catalog, ['packs', 'activeId', 'templates']) || !Array.isArray(v.catalog.packs) || !Array.isArray(v.catalog.templates) || !profileId(v.catalog.activeId) ||
    !object(v.definitions) || !Array.isArray(v.definitions.actions) || !Array.isArray(v.definitions.events) || !Array.isArray(v.definitions.plugins) || !Array.isArray(v.definitions.actionTypes)) return invalid();
  const profiles = new Set<string>();
  const members = new Set<string>();
  for (const pack of v.catalog.packs) {
    if (!exact(pack, ['id', 'name', 'description', 'eventIds', 'actionIds']) || !profileId(pack.id) || pack.id === 'default' || profiles.has(pack.id) ||
      typeof pack.name !== 'string' || !pack.name.trim() || typeof pack.description !== 'string' || !Array.isArray(pack.eventIds) || !Array.isArray(pack.actionIds)) return invalid();
    profiles.add(pack.id);
    for (const [kind, ids] of [['event', pack.eventIds], ['action', pack.actionIds]] as const) for (const member of ids) {
      if (!id(member) || members.has(`${kind}:${member}`)) return invalid();
      members.add(`${kind}:${member}`);
    }
  }
  if (v.catalog.activeId !== 'default' && !profiles.has(v.catalog.activeId)) return invalid();
  const templates = new Set<string>();
  for (const template of v.catalog.templates) {
    if (!object(template) || !templateValue(template) || !profileId(template.id) || templates.has(template.id)) return invalid();
    templates.add(template.id);
  }
  const versions = parseVersions(v.resourceVersions, false);
  const available = new Map(versions.map((version) => [`${version.scope}:${version.id}`, version.revision]));
  let count = 0;
  for (const [kind, records] of [['action', v.definitions.actions], ['event', v.definitions.events]] as const) {
    const ids = new Set<string>();
    for (const record of records) {
      if (!object(record) || !id(record.id) || ids.has(record.id)) return invalid();
      assertRecord(record as never, kind, record.id);
      if (!available.get(`${kind}:${record.id}`) || available.get(`${kind}:${record.id}`) === '0') return invalid();
      ids.add(record.id); ++count;
    }
  }
  if (count !== versions.length) return invalid();
  if (!sourceNumbersSafe(v.definitions)) throw new ControlCallError('source_precision', 'The saved library contains integers this browser cannot display exactly.');
  if (libraryDefinitionDigest(v.definitions as unknown as BehaviorSnapshot, versions) !== v.definitionDigest) return invalid();
  return v as unknown as LibrarySnapshot;
}
export function checkLibraryMutation(mutation: LibraryMutation): LibraryMutation {
  if (!sourceNumbersSafe(mutation)) throw new ControlCallError('unsafe_source', 'This browser cannot write source containing integers outside its exact range.');
  const cloned = JSON.parse(JSON.stringify(mutation)) as LibraryMutation;
  if (!libraryOperations.includes(cloned.operation) || new TextEncoder().encode(JSON.stringify(cloned)).length > 8 * 1024 * 1024) throw new ControlCallError('too_large', 'The library change exceeds its bounded request size.');
  const fail = (): never => { throw new ControlCallError('invalid_params', 'Invalid reviewed library change.'); };
  const required = {
    'create-profile': ['operation', 'id', 'name', 'description'], 'switch-profile': ['operation', 'id'], 'delete-profile': ['operation', 'id'],
    'apply-template': ['operation', 'entry', 'profileId'], 'import-profile': ['operation', 'profile', 'entries'],
    'assign-record': ['operation', 'kind', 'id', 'profileId'], 'import-templates': ['operation', 'templates'], 'delete-template': ['operation', 'id'],
  }[cloned.operation];
  const optional = cloned.operation === 'apply-template' ? ['additionalEntries', 'templates'] : cloned.operation === 'import-profile' ? ['templates'] : [];
  if (!required.every((key) => Object.hasOwn(cloned, key)) || Object.keys(cloned).some((key) => !required.includes(key) && !optional.includes(key))) fail();
  if ('id' in cloned && (cloned.operation === 'assign-record' ? !id(cloned.id) : !profileId(cloned.id))) fail();
  if ('profileId' in cloned && !profileId(cloned.profileId)) fail();
  if (cloned.operation === 'assign-record' && !['action', 'event'].includes(cloned.kind)) fail();
  const descriptor = cloned.operation === 'create-profile' ? cloned : cloned.operation === 'import-profile' ? cloned.profile : null;
  if (descriptor && (!profileId(descriptor.id) || descriptor.id === 'default' || typeof descriptor.name !== 'string' || !rustTrim(descriptor.name) || Array.from(descriptor.name).length > 80 || typeof descriptor.description !== 'string' || Array.from(descriptor.description).length > 500)) fail();
  if (cloned.operation === 'import-profile' && (!exact(cloned.profile, ['id', 'name', 'description']) || !Array.isArray(cloned.entries))) fail();
  if (cloned.operation === 'apply-template' && cloned.additionalEntries !== undefined && (!Array.isArray(cloned.additionalEntries) || cloned.additionalEntries.length > 31)) fail();
  const source = (record: unknown): void => {
    const pending = [{ value: record, depth: 0 }]; let nodes = 0;
    while (pending.length) {
      const next = pending.pop()!;
      if (next.depth > 64 || ++nodes > 65_536) fail();
      if (next.value && typeof next.value === 'object') for (const value of Object.values(next.value)) pending.push({ value, depth: next.depth + 1 });
    }
    if (new TextEncoder().encode(JSON.stringify(record)).length > 256 * 1024) throw new ControlCallError('too_large', 'A source record exceeds its bounded size.');
  };
  const entries = cloned.operation === 'apply-template' ? [cloned.entry, ...(cloned.additionalEntries ?? [])] : cloned.operation === 'import-profile' ? cloned.entries : [];
  let records = 0; const seen = new Set<string>();
  for (const entry of entries) {
    if (!exact(entry, ['actions', 'event']) || !Array.isArray(entry.actions) || entry.actions.length < 1 || entry.actions.length > 8) fail();
    for (const [kind, rows] of [['action', entry.actions], ['event', [entry.event]]] as const) for (const record of rows) {
      source(record);
      if (!object(record) || !id(record.id) || typeof record.name !== 'string' || !rustTrim(record.name) || typeof record.enabled !== 'boolean' || seen.has(`${kind}:${record.id}`) || ++records > 512) fail();
      assertRecord(record as never, kind, record.id); seen.add(`${kind}:${record.id}`);
    }
    if (stableLibraryJson(entry.event.actionIds) !== stableLibraryJson(entry.actions.map((action) => action.id))) fail();
  }
  if (entries.length > 32 || cloned.operation === 'import-profile' && entries.length === 0) fail();
  const templates = cloned.operation === 'import-templates' || cloned.operation === 'import-profile' || cloned.operation === 'apply-template' ? cloned.templates : undefined;
  if (templates !== undefined) {
    if (!Array.isArray(templates) || templates.length > 128 || cloned.operation === 'import-templates' && templates.length === 0) fail();
    templates.forEach((template) => { if (!templateValue(template)) fail(); source(template); });
  }
  return cloned;
}
export function parseLibrarySubmission(v: unknown, mutation: LibraryMutation, context: LibrarySnapshot): LibrarySubmission {
  if (!exact(v, ['state', 'acknowledgement'])) return invalid();
  const state = parseLibraryState(v.state);
  if (!sameLibraryHost(context, state)) return invalid();
  if (v.acknowledgement === null) return { state, acknowledgement: null };
  const a = v.acknowledgement; const result = state.recent[0];
  if (!result || result.outcome !== 'succeeded' || !result.receipt || state.pending || result.intent.operation !== mutation.operation || result.intent.target !== libraryTarget(mutation) ||
    result.intent.expectedRevision !== context.revision || result.intent.definitionDigest !== context.definitionDigest || result.intent.commandHash !== libraryCommandHash(mutation, context) || result.intent.hostInstanceId !== context.hostInstanceId ||
    !exact(a, ['value', 'resourceVersions', 'command', 'hostInstanceId']) || a.hostInstanceId !== context.hostInstanceId ||
    !exact(a.command, ['commandId', 'state', 'resource', 'committedAt']) || a.command.commandId !== result.intent.commandId || a.command.state !== 'succeeded' || !time(a.command.committedAt) || a.command.committedAt === 0 ||
    !exact(a.command.resource, ['kind', 'scope', 'id']) || a.command.resource.kind !== 'automation' || a.command.resource.scope !== 'library' || a.command.resource.id !== 'catalog' ||
    stableLibraryJson(a.value) !== stableLibraryJson(result.receipt)) return invalid();
  const versions = parseVersions(a.resourceVersions, true);
  if (versions.length !== result.receipt.affectedResources || versions.find((version) => version.scope === 'library')?.revision !== result.receipt.revision || libraryDigest(versions) !== result.receipt.versionsDigest) return invalid();
  return { state, acknowledgement: a as unknown as LibraryAcknowledgement };
}
