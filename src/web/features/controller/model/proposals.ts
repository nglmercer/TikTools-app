import { computed, ref, watch, type Ref } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import type { ControllerStatus } from './controller.ts';
import type { AutomationSummary } from './automations.ts';

type Family = 'event' | 'action';
type Mode = 'create' | 'update';
interface Identity { epoch: string; connectionId: string; hostInstanceId: string; grantRevision: string }
export interface ProposalDraft { kind: Family; id: string; mode: Mode; baseRevision: string; text: string; updatedAt: number }
interface ProposalRow extends Omit<ProposalDraft, 'text'> { textBytes: number }
interface Inspection extends Identity { inspectionId: string; kind: Family; id: string; revision: string; summary: AutomationSummary | null }
export interface ProposalConfirmation extends Identity {
  operation: 'save' | 'send' | 'send-active' | 'remove' | 'base'; binding: string; origin: string; hostId: string;
  kind: Family; id: string; mode: Mode; baseRevision: string; text: string; storeRevision: string; inspectionId: string | null;
}
interface Dependencies { status: Ref<ControllerStatus>; blocked: () => boolean; journalMatches: () => boolean; send: (method: string, params: Record<string, unknown>) => Promise<boolean | { journal: unknown }> }
const encoder = new TextEncoder();
const revision = (v: unknown): v is string => typeof v === 'string' && /^(0|[1-9]\d{0,18})$/.test(v) && BigInt(v) <= 9223372036854775807n;
const canonical = (id: string) => !!id && id.trim() === id && encoder.encode(id).length <= 128 && !/[\p{Cc}*]/u.test(id);
function binding(s: ControllerStatus): string {
  const h = s.hello;
  return JSON.stringify([s.phase, s.epoch, s.profile, s.origin, h?.connectionId, h?.hostId, h?.hostInstanceId, h?.principalId, h?.sessionId, h?.grantRevision, h?.capabilities, h?.resourceRestrictions]);
}
function identity(s: ControllerStatus): Identity | null {
  const h = s.hello;
  return s.phase === 'ready' && !!h?.connectionId && !!h.grantRevision ? { epoch: s.epoch, connectionId: h.connectionId, hostInstanceId: h.hostInstanceId, grantRevision: h.grantRevision } : null;
}
function tuple(reply: Identity, expected: Identity): boolean { return reply.epoch === expected.epoch && reply.connectionId === expected.connectionId && reply.hostInstanceId === expected.hostInstanceId && reply.grantRevision === expected.grantRevision; }

/** Full text comes only from this editor or its explicitly retained native copy. */
export function createProposalModel(control: ControlClient, deps: Dependencies) {
  const kind = ref<Family>('action'); const id = ref(''); const mode = ref<Mode>('create'); const baseRevision = ref('0'); const text = ref('');
  const rows = ref<ProposalRow[]>([]); const saved = ref<ProposalDraft | null>(null); const inspection = ref<Inspection | null>(null);
  const sharedSource = ref<{ kind: Family; id: string; revision: string; text: string; before: string } | null>(null);
  const storeRevision = ref<string | null>(null); const busy = ref(false); const error = ref(''); let generation = 0; let loaded = ''; let disposed = false;
  const identityKey = computed(() => binding(deps.status.value));
  const allowed = computed(() => deps.status.value.proposalsSupported === true && !!identity(deps.status.value) && !!deps.status.value.hello?.capabilities.includes('automation.read') && !Object.hasOwn(deps.status.value.hello?.resourceRestrictions ?? {}, 'live'));
  const permitted = (family: Family, target: string) => allowed.value && canonical(target) && (deps.status.value.hello?.resourceRestrictions?.['automation'] === undefined || deps.status.value.hello.resourceRestrictions['automation']!.includes(`${family}:${target}`));
  const editable = computed(() => permitted(kind.value, id.value));
  const matches = computed(() => !!saved.value && saved.value.kind === kind.value && saved.value.id === id.value && saved.value.mode === mode.value && saved.value.baseRevision === baseRevision.value && saved.value.text === text.value);
  const inspected = computed(() => !!inspection.value && inspection.value.kind === kind.value && inspection.value.id === id.value && inspection.value.revision === baseRevision.value && (inspection.value.summary !== null) === (mode.value === 'update'));
  const canSubmit = computed(() => editable.value && !busy.value && storeRevision.value !== null && encoder.encode(text.value).length <= 256 * 1024 && (inspection.value === null || inspected.value) && !deps.blocked() && deps.journalMatches() && deps.status.value.hello?.capabilities.includes('automation.edit') === true && deps.status.value.hello.features['automationDrafts'] === true);
  const canSend = computed(() => canSubmit.value && matches.value && inspected.value);
  const canReadShared = computed(() => editable.value && !busy.value && deps.status.value.hello?.capabilities.includes('automation.edit') === true && deps.status.value.hello.features['automationSharedSource'] === true);
  const canSendActive = computed(() => canSend.value && deps.status.value.hello?.features['automationReusableWrite'] === true && deps.status.value.hello.capabilities.includes('automation.toggle'));
  const canSubmitActive = computed(() => canSubmit.value && deps.status.value.hello?.features['automationReusableWrite'] === true && deps.status.value.hello.capabilities.includes('automation.toggle'));
  function reset(): void { generation += 1; rows.value = []; saved.value = null; inspection.value = null; sharedSource.value = null; storeRevision.value = null; text.value = ''; id.value = ''; baseRevision.value = '0'; mode.value = 'create'; error.value = ''; busy.value = false; loaded = ''; }
  const unwatch = watch(identityKey, reset, { flush: 'sync' });
  function active(g: number, b: string): boolean { return !disposed && generation === g && identityKey.value === b && allowed.value; }
  async function local<T>(method: string, params: Record<string, unknown>, apply: (reply: T) => void): Promise<void> {
    if (!allowed.value || busy.value) return;
    const fence = identity(deps.status.value)!; const b = identityKey.value; const g = generation;
    busy.value = true; error.value = '';
    try {
      const reply = await control.call<T>(method, { ...fence, ...params });
      if (!active(g, b)) return;
      if (!tuple(reply as Identity, fence)) throw new Error('stale_view');
      apply(reply);
    } catch (failure) { if (active(g, b)) error.value = failure instanceof Error ? failure.message : 'proposal_unavailable'; }
    finally { if (active(g, b)) busy.value = false; }
  }
  async function refresh(): Promise<void> {
    if (!allowed.value || busy.value || loaded === identityKey.value) return;
    const b = identityKey.value; const g = generation; const fence = identity(deps.status.value)!;
    busy.value = true; error.value = '';
    try {
      let offset: number | null = 0; let revisionValue: string | undefined; const next: ProposalRow[] = []; const seen = new Set<string>();
      while (offset !== null) {
        const page: Identity & { storeRevision: string; rows: ProposalRow[]; next: number | null } = await control.call('controller.proposals.list', { ...fence, offset });
        if (!active(g, b)) return;
        if (!tuple(page, fence) || !revision(page.storeRevision) || (revisionValue !== undefined && revisionValue !== page.storeRevision) || !Array.isArray(page.rows) || page.rows.length > 32 || next.length + page.rows.length > 128 || (page.next !== null && (!Number.isSafeInteger(page.next) || page.next <= offset || !page.rows.length))) throw new Error('proposal_invalid');
        for (const row of page.rows) {
          if ((row.kind !== 'event' && row.kind !== 'action') || !permitted(row.kind, row.id) || !revision(row.baseRevision) || !['create', 'update'].includes(row.mode) || !Number.isSafeInteger(row.updatedAt) || !Number.isSafeInteger(row.textBytes) || row.textBytes < 0 || row.textBytes > 256 * 1024 || seen.has(`${row.kind}:${row.id}`)) throw new Error('proposal_invalid');
          seen.add(`${row.kind}:${row.id}`); next.push(row);
        }
        revisionValue = page.storeRevision; offset = page.next;
      }
      rows.value = next;
      // A catalog refresh must not silently give an older open editor a new
      // replacement precondition. Only an explicit selected read/save may do so.
      if (!id.value || storeRevision.value === null || storeRevision.value === revisionValue) storeRevision.value = revisionValue!;
      else error.value = 'proposal_conflict';
      loaded = b;
    } catch (failure) { if (active(g, b)) error.value = failure instanceof Error ? failure.message : 'proposal_unavailable'; }
    finally { if (active(g, b)) busy.value = false; }
  }
  async function reloadInventory(): Promise<void> { loaded = ''; await refresh(); }
  function select(family: Family, target: string): void {
    if (!permitted(family, target) || busy.value) return;
    generation += 1; kind.value = family; id.value = target; mode.value = 'create'; baseRevision.value = '0'; saved.value = null; inspection.value = null; sharedSource.value = null;
    text.value = JSON.stringify(family === 'action' ? { id: target, name: '', enabled: false, typeId: 'core.log', config: {}, schemaVersion: 2 } : { id: target, name: '', enabled: false, trigger: 'tiktok.chat', actionIds: [], filters: [], cooldownMs: 0, cooldownScope: 'global', runMode: 'all', schemaVersion: 1 }, null, 2);
  }
  async function load(): Promise<void> {
    if (!editable.value) return;
    const target = { kind: kind.value, id: id.value };
    await local<Identity & { storeRevision: string; draft: ProposalDraft | null }>('controller.proposals.get', target, (r) => {
      if (!revision(r.storeRevision)) throw new Error('proposal_invalid');
      if (r.draft && (r.draft.kind !== target.kind || r.draft.id !== target.id || !revision(r.draft.baseRevision) || !['create', 'update'].includes(r.draft.mode) || typeof r.draft.text !== 'string' || encoder.encode(r.draft.text).length > 256 * 1024)) throw new Error('proposal_invalid');
      storeRevision.value = r.storeRevision; saved.value = r.draft; inspection.value = null;
      if (r.draft) { text.value = r.draft.text; mode.value = r.draft.mode; baseRevision.value = r.draft.baseRevision; }
    });
  }
  async function inspect(): Promise<void> {
    if (!editable.value) return;
    const target = { kind: kind.value, id: id.value };
    inspection.value = null;
    await local<Inspection>('controller.proposals.inspect', target, (r) => {
      if (r.kind !== target.kind || r.id !== target.id || !revision(r.revision) || typeof r.inspectionId !== 'string' || !r.inspectionId || (r.summary !== null && (Object.keys(r.summary).length !== 5 || r.summary.kind !== target.kind || r.summary.id !== target.id || typeof r.summary.name !== 'string' || [...r.summary.name].length > 160 || typeof r.summary.enabled !== 'boolean' || r.summary.definitionRedacted !== true || r.revision === '0'))) throw new Error('protocol');
      inspection.value = r;
    });
  }
  async function loadShared(): Promise<void> {
    if (!canReadShared.value) return;
    const target = { kind: kind.value, id: id.value }; const before = text.value;
    sharedSource.value = null;
    await local<Identity & { kind: Family; id: string; revision: string; record: unknown }>('controller.proposals.source', target, (reply) => {
      if (reply.kind !== target.kind || reply.id !== target.id || !revision(reply.revision) || reply.revision === '0' ||
          !reply.record || typeof reply.record !== 'object' || Array.isArray(reply.record) ||
          (reply.record as Record<string, unknown>)['id'] !== target.id || typeof (reply.record as Record<string, unknown>)['enabled'] !== 'boolean') throw new Error('protocol');
      const record = { ...(reply.record as Record<string, unknown>), enabled: false };
      const source = JSON.stringify(record, null, 2);
      if (encoder.encode(source).length > 256 * 1024) throw new Error('proposal_invalid');
      sharedSource.value = { ...target, revision: reply.revision, text: source, before };
    });
  }
  function useShared(): void {
    const source = sharedSource.value;
    if (!source || !canReadShared.value || source.kind !== kind.value || source.id !== id.value || source.before !== text.value) return;
    text.value = source.text; baseRevision.value = source.revision; mode.value = 'update';
    inspection.value = null; sharedSource.value = null;
  }
  function edit(value: string): void { if (editable.value && !busy.value) text.value = value; }
  function prepare(operation: ProposalConfirmation['operation']): ProposalConfirmation | null {
    const s = deps.status.value; const fence = identity(s);
    if (!editable.value || busy.value || !fence || !s.origin || !s.hello || storeRevision.value === null || !revision(baseRevision.value) || (mode.value === 'update' && baseRevision.value === '0') || encoder.encode(text.value).length > 256 * 1024 || (operation === 'send' && !canSend.value) || (operation === 'send-active' && !canSendActive.value) || (operation === 'remove' && !saved.value) || (operation === 'base' && !inspection.value)) return null;
    return { ...fence, operation, binding: identityKey.value, origin: s.origin, hostId: s.hello.hostId, kind: kind.value, id: id.value, mode: mode.value, baseRevision: baseRevision.value, text: text.value, storeRevision: storeRevision.value, inspectionId: inspection.value?.inspectionId ?? null };
  }
  function current(t: ProposalConfirmation): boolean {
    const now = prepare(t.operation);
    return now !== null && JSON.stringify(now) === JSON.stringify(t);
  }
  async function confirm(t: ProposalConfirmation): Promise<void> {
    if (!current(t)) return;
    if (t.operation === 'base') { const i = inspection.value!; baseRevision.value = i.revision; mode.value = i.summary === null ? 'create' : 'update'; error.value = ''; return; }
    if (t.operation === 'send' || t.operation === 'send-active') {
      const sentGeneration = generation;
      const accepted = await deps.send('controller.proposals.submit', { epoch: t.epoch, connectionId: t.connectionId, hostInstanceId: t.hostInstanceId, grantRevision: t.grantRevision, kind: t.kind, id: t.id, inspectionId: t.inspectionId, expectedStoreRevision: t.storeRevision, acknowledgeProposal: true, ...(t.operation === 'send-active' ? { active: true } : {}) });
      if (accepted && identityKey.value === t.binding && generation === sentGeneration) {
        // Only the exact native response for this submission can advance the
        // open base. A later recovered receipt or unrelated command is not
        // proof that the current editor still represents its submitted body.
        const journal = typeof accepted === 'object' ? accepted.journal : null;
        const value = journal && typeof journal === 'object' && !Array.isArray(journal) ? journal as Record<string, unknown> : null;
        const bound = value?.['binding'] as Record<string, unknown> | undefined;
        const recent = Array.isArray(value?.['recent']) ? value['recent'][0] as Record<string, unknown> | undefined : undefined;
        const intent = recent?.['intent'] as Record<string, unknown> | undefined;
        const host = deps.status.value.hello;
        const expectedOperation = `${t.mode}-${t.kind}-${t.operation === 'send-active' ? 'active' : 'draft'}`;
        const resultRevision = recent?.['revision'];
        if (deps.journalMatches() && value?.['pending'] === null && value?.['unavailable'] === false &&
          bound?.['origin'] === t.origin && bound?.['hostId'] === t.hostId && bound?.['principalId'] === host?.principalId && bound?.['sessionId'] === host?.sessionId &&
          bound?.['profileId'] === deps.status.value.commandJournal?.binding.profileId &&
          recent?.['outcome'] === 'succeeded' && intent?.['operation'] === expectedOperation && intent['resourceId'] === t.id && intent['hostInstanceId'] === t.hostInstanceId &&
          intent['expectedRevision'] === t.baseRevision && intent['proposalRevision'] === t.storeRevision && revision(resultRevision) && BigInt(resultRevision) === BigInt(t.baseRevision) + 1n &&
          kind.value === t.kind && id.value === t.id && mode.value === t.mode && baseRevision.value === t.baseRevision && storeRevision.value === t.storeRevision &&
          saved.value?.text === t.text && saved.value.kind === t.kind && saved.value.id === t.id && saved.value.mode === t.mode && saved.value.baseRevision === t.baseRevision) {
          mode.value = 'update'; baseRevision.value = resultRevision;
        }
        inspection.value = null;
      }
      return;
    }
    const params = { kind: t.kind, id: t.id, expectedStoreRevision: t.storeRevision, ...(t.operation === 'save' ? { mode: t.mode, baseRevision: t.baseRevision, text: t.text, acknowledgeStorage: true } : {}) };
    await local<Identity & { storeRevision: string }>(t.operation === 'save' ? 'controller.proposals.save' : 'controller.proposals.delete', params, (r) => {
      if (!revision(r.storeRevision) || BigInt(r.storeRevision) !== BigInt(t.storeRevision) + 1n) throw new Error('proposal_invalid');
      storeRevision.value = r.storeRevision; saved.value = t.operation === 'save' ? { kind: t.kind, id: t.id, mode: t.mode, baseRevision: t.baseRevision, text: t.text, updatedAt: Date.now() } : null; loaded = '';
    });
    await refresh();
  }
  async function submit(active: boolean): Promise<void> {
    if (!(active ? canSubmitActive.value : canSubmit.value)) return;
    let initial = prepare('save');
    if (!initial || !current(initial)) return;
    if (!inspection.value) {
      const before = initial;
      await inspect();
      // The host read may complete after the author or selected profile has
      // changed. It must never silently choose a different replacement base.
      if (identityKey.value !== before.binding || kind.value !== before.kind || id.value !== before.id || mode.value !== before.mode || baseRevision.value !== before.baseRevision || text.value !== before.text || !inspection.value) return;
      if (!inspected.value) { error.value = 'proposal_base_changed'; return; }
      initial = prepare('save');
      if (!initial || !current(initial)) return;
    }
    if (!matches.value) await confirm(initial);
    // A save may be delayed while the session, permission, inspection or text
    // changes. Never dispatch a different proposal than the one selected.
    if (identityKey.value !== initial.binding || kind.value !== initial.kind || id.value !== initial.id || mode.value !== initial.mode || baseRevision.value !== initial.baseRevision || text.value !== initial.text || inspection.value?.inspectionId !== initial.inspectionId || error.value) return;
    const send = prepare(active ? 'send-active' : 'send');
    if (send && current(send)) await confirm(send);
  }
  function stop(): void { disposed = true; unwatch(); reset(); }
  return { kind, id, mode, baseRevision, text, rows, saved, inspection, sharedSource, storeRevision, busy, error, identityKey, allowed, editable, matches, inspected, canSend, canSendActive, canSubmit, canSubmitActive, canReadShared, refresh, reloadInventory, reset, select, load, loadShared, useShared, inspect, edit, prepare, current, confirm, submit, stop };
}
export type ProposalModel = ReturnType<typeof createProposalModel>;
