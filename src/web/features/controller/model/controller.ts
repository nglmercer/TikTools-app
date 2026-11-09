import { computed, ref } from 'vue';
import type { ControlClient, ResourceVersion } from '@/shared/api/control-client.ts';
import { createDraftPersistence, type LocalDraft } from './drafts.ts';
import { createAutomationModel } from './automations.ts';
import { createProposalModel } from './proposals.ts';
import { createLiveStatusModel } from './live.ts';
import { createLiveFeedModel } from './live-feed.ts';
import { createCompetitionModel } from './competitions.ts';

export interface ControllerStatus {
  draftsSupported?: boolean;
  proposalsSupported?: boolean;
  browserJournalMissing?: boolean;
  epoch: string; viewRevision: string; phase: string; profile: string | null; origin: string | null; errorCode: string | null;
  automationRevision?: string;
  pointsRevision?: string; pointsLoading?: boolean; pointsErrorCode?: string | null;
  pairingRequestId?: string | null;
  commandJournal?: CommandJournal | null; commandBusy?: boolean; commandErrorCode?: string | null;
  hello: { connectionId?: string; grantRevision?: string; hostId: string; principalId: string; sessionId: string; hostInstanceId: string; capabilities: string[]; features: Record<string, boolean>; resourceRestrictions?: Record<string, string[]>; limits?: { requestsPerSecond: number; requestBurst: number } } | null;
}
export type WorkflowOperation = 'disable' | 'delete';
export interface PendingCommand { commandId: string; operation: WorkflowOperation | 'pause-competition' | 'cancel-competition' | 'adjust-points' | 'disable-event' | 'delete-event' | 'enable-event' | 'enable-action' | 'create-event-draft' | 'update-event-draft' | 'create-action-draft' | 'update-action-draft' | 'create-event-active' | 'update-event-active' | 'create-action-active' | 'update-action-active'; resourceId: string; hostInstanceId: string; expectedRevision?: string; expectedGeneration?: string; delta?: number; currencyName?: string; proposalRevision?: string; createdAt: number }
export interface CommandJournal {
  binding: { profileId: string; origin: string; hostId: string; principalId: string; sessionId: string };
  pending: PendingCommand | null; unavailable: boolean;
  recent: Array<{ intent: PendingCommand; outcome: 'succeeded' | 'rejected' | 'reviewed-unknown'; revision: string | null; errorCode: string | null; recordedAt: number; pointsResult?: { delta: number; totalPoints: number; level: number } }>;
}
export interface CommandTarget { epoch: string; viewRevision: string; id: string; expectedRevision: string; operation: WorkflowOperation; origin: string; hostId: string; principalId: string; sessionId: string }
export interface ReviewTarget { epoch: string; commandId: string; binding: string }
export interface WorkflowRow { graph: Record<string, unknown>; version: ResourceVersion }
export interface PointRow { viewer: { uniqueId: string; nickname?: string; points: number; level: number }; version: ResourceVersion }
export interface PointsPage { epoch: string; pointsRevision: string; loading: boolean; errorCode: string | null; currencyName: string | null; rows: PointRow[] | null; selectedId: string | null; selected: PointRow | null; selectedVersion: ResourceVersion | null }
export interface PointsTarget { epoch: string; connectionId: string; origin: string; hostId: string; principalId: string; sessionId: string; hostInstanceId: string; grantRevision?: string; uniqueId: string; delta: number; currencyName: string; creates: boolean }
export interface RemoteProfile { name: string; profileId: string; origin: string; hostId: string }
export type ProfileForgetTarget = Readonly<RemoteProfile>;
interface Page { epoch: string; viewRevision: string; rows: WorkflowRow[]; next: number | null }
const idle = (): ControllerStatus => ({ epoch: '0', viewRevision: '0', phase: 'idle', profile: null, origin: null, hello: null, errorCode: null });

/** One instance per controller window. Owner stores are never initialized. */
export function createControllerModel(control: ControlClient) {
  const status = ref(idle());
  const profiles = ref<RemoteProfile[]>([]);
  const rows = ref<WorkflowRow[]>([]);
  const points = ref<PointsPage | null>(null);
  const pointsError = ref('');
  const pointsLoaded = ref('');
  const error = ref('');
  const busy = ref(false);
  const commandSending = ref(false);
  const loaded = ref('');
  const scope = ref('');
  const drafts = new Map<string, Map<string, LocalDraft>>();
  const draftRevision = ref(0);
  let stopped = false;
  let refreshing: Promise<void> | undefined;
  let dirty = false;
  let selection = 0;
  let timer: ReturnType<typeof setInterval> | undefined;
  let unsubscribe: (() => void) | undefined;
  let nextLowRatePoll = 0;
  let lowRateTurn = 0;
  let readBinding = '';
  const key = (s: ControllerStatus) => `${s.epoch}:${s.viewRevision}`;
  const ready = computed(() => status.value.phase === 'ready' && loaded.value === key(status.value));
  const pointsAllowed = computed(() => !!status.value.hello?.features['pointsProjection'] && status.value.hello.capabilities.includes('points.read') && !Object.prototype.hasOwnProperty.call(status.value.hello.resourceRestrictions ?? {}, 'live'));
  const pointsKey = (s: ControllerStatus) => `${s.epoch}:${s.pointsRevision ?? ''}`;
  async function refreshPoints(current: ControllerStatus, generation: number): Promise<void> {
    if (!pointsAllowed.value || current.phase !== 'ready') { points.value = null; pointsLoaded.value = ''; return; }
    if (current.pointsRevision === undefined || pointsLoaded.value === pointsKey(current)) return;
    try {
      const page = await control.call<PointsPage>('controller.points', { epoch: current.epoch, pointsRevision: current.pointsRevision });
      const after = await control.call<ControllerStatus>('controller.status');
      if (stopped || generation !== selection) return;
      if (after.phase !== 'ready' || pointsKey(after) !== pointsKey(current) || page.epoch !== current.epoch || page.pointsRevision !== current.pointsRevision) {
        points.value = null; pointsLoaded.value = ''; dirty = true; return;
      }
      points.value = page; pointsLoaded.value = pointsKey(current); pointsError.value = page.errorCode ?? '';
    } catch (failure) {
      if (stopped || generation !== selection) return;
      points.value = null; pointsLoaded.value = ''; pointsError.value = failure instanceof Error ? failure.message : 'Points view unavailable';
    }
  }
  async function refresh(): Promise<void> {
    if (stopped) return;
    dirty = true;
    if (refreshing) return refreshing;
    refreshing = (async () => {
      while (dirty && !stopped) {
        dirty = false;
        const generation = selection;
        try {
          const current = await control.call<ControllerStatus>('controller.status');
          if (stopped) return;
          if (generation !== selection) { dirty = true; continue; }
          if (current.epoch !== status.value.epoch || current.phase !== 'ready') { rows.value = []; loaded.value = ''; points.value = null; pointsLoaded.value = ''; draftStorage.reset(); }
          const pairingFinished = status.value.phase === 'pairing' && current.phase === 'idle';
          status.value = current;
          const currentReadBinding = JSON.stringify([current.epoch, current.phase, current.profile, current.origin, current.hello]);
          if (currentReadBinding !== readBinding) {
            // Never keep the previous scope visible while low-rate polling
            // waits for its next turn.
            live.clear(); liveFeed.clear(); competitions.clear();
            nextLowRatePoll = 0; lowRateTurn = 0; readBinding = currentReadBinding;
          }
          const rate = current.hello?.limits?.requestsPerSecond ?? 30;
          if (rate <= 2 && competitions.allowed.value) {
            // Share a small request budget across optional live/competition
            // reads, leaving capacity for the native heartbeat and points.
            if (performance.now() >= nextLowRatePoll) {
              const reads = [live, liveFeed, competitions];
              for (let offset = 0; offset < reads.length; offset += 1) {
                const index = (lowRateTurn + offset) % reads.length;
                const read = reads[index]!;
                if (!read.allowed.value) continue;
                lowRateTurn = (index + 1) % reads.length;
                nextLowRatePoll = performance.now() + (rate <= 1 ? 3000 : 1500);
                void read.refresh(current, generation);
                break;
              }
            }
          } else {
            void live.refresh(current, generation);
            void liveFeed.refresh(current, generation);
            void competitions.refresh(current, generation);
          }
          void proposals.refresh();
          if (pairingFinished) await inventory();
          if (current.hello) scope.value = JSON.stringify([current.origin, current.hello.hostId, current.hello.principalId, current.hello.sessionId]);
          else if (current.phase === 'connecting' || current.phase === 'pairing') scope.value = '';
          await refreshPoints(current, generation);
          if (automation.allowed.value && current.automationRevision !== undefined) await automation.refresh(current, generation);
          else automation.clear();
          if (current.phase !== 'ready') continue;
          if (loaded.value === key(current)) { await draftStorage.refresh(current, generation); continue; }
          const nextRows: WorkflowRow[] = [];
          const positions = new Set<number>();
          let offset: number | null = 0;
          while (offset !== null) {
            if (positions.has(offset) || positions.size > 4096) throw new Error('Invalid controller pagination');
            positions.add(offset);
            const page: Page = await control.call<Page>('controller.workflows', { epoch: current.epoch, viewRevision: current.viewRevision, offset });
            if (page.epoch !== current.epoch || page.viewRevision !== current.viewRevision) throw new Error('Controller view changed');
            nextRows.push(...page.rows); offset = page.next;
          }
          const after = await control.call<ControllerStatus>('controller.status');
          if (stopped) return;
          if (generation !== selection) { dirty = true; continue; }
          if (key(current) !== key(after) || after.phase !== 'ready') { rows.value = []; loaded.value = ''; status.value = after; dirty = true; continue; }
          rows.value = nextRows; loaded.value = key(current); error.value = '';
          await draftStorage.refresh(current, generation);
        } catch (failure) {
          if (stopped) return;
          if (generation !== selection) { dirty = true; continue; }
          rows.value = []; loaded.value = ''; live.clear(); liveFeed.clear(); competitions.clear();
          error.value = failure instanceof Error ? failure.message : 'Controller unavailable';
          // Next native change/heartbeat retries the read, never a mutation.
        }
      }
    })().finally(() => { refreshing = undefined; });
    return refreshing;
  }
  async function inventory(): Promise<void> { profiles.value = await control.call<RemoteProfile[]>('controller.profiles'); }
  function profileForgetCurrent(target: ProfileForgetTarget): boolean {
    return !busy.value && !commandSending.value && status.value.profile !== target.name &&
      !!target.profileId && profiles.value.some((profile) => profile.name === target.name &&
        profile.profileId === target.profileId && profile.origin === target.origin && profile.hostId === target.hostId);
  }
  function prepareForget(profile: RemoteProfile): ProfileForgetTarget | null {
    const target = { name: profile.name, profileId: profile.profileId, origin: profile.origin, hostId: profile.hostId };
    return profileForgetCurrent(target) ? target : null;
  }
  async function forgetProfile(target: ProfileForgetTarget): Promise<boolean> {
    if (!profileForgetCurrent(target) || stopped) return false;
    const generation = selection;
    busy.value = true; error.value = '';
    try {
      await control.call('controller.profile.forget', { profile: target.name, profileId: target.profileId, origin: target.origin, hostId: target.hostId, confirm: true });
      if (!stopped && generation === selection) await inventory();
      return true;
    } catch (failure) {
      if (!stopped && generation === selection) error.value = failure instanceof Error ? failure.message : 'Profile removal failed';
      return false;
    } finally { if (!stopped && generation === selection) busy.value = false; }
  }
  async function connect(profile: string): Promise<void> {
    const generation = ++selection;
    draftStorage.reset(); automation.clear(); live.clear(); liveFeed.clear(); competitions.clear();
    busy.value = true; rows.value = []; points.value = null; pointsLoaded.value = ''; loaded.value = ''; scope.value = ''; status.value = { ...idle(), phase: 'connecting', profile };
    try { await control.call('controller.connect', { profile }); if (generation === selection) await refresh(); }
    catch (failure) { if (generation === selection) { error.value = failure instanceof Error ? failure.message : 'Connection failed'; await refresh(); } }
    finally { if (generation === selection) busy.value = false; }
  }
  async function disconnect(): Promise<void> {
    selection += 1;
    draftStorage.reset(); automation.clear(); live.clear(); liveFeed.clear(); competitions.clear();
    busy.value = true; loaded.value = ''; rows.value = []; points.value = null; pointsLoaded.value = '';
    try { await control.call('controller.disconnect'); await refresh(); } finally { busy.value = false; }
  }
  function draft(id: string): LocalDraft | undefined { void draftRevision.value; return drafts.get(scope.value)?.get(id); }
  function putDraft(id: string, value: LocalDraft): void {
    let own = drafts.get(scope.value); if (!own) { own = new Map(); drafts.set(scope.value, own); }
    own.set(id, value); draftRevision.value += 1;
  }
  function dropDraft(id: string): void { drafts.get(scope.value)?.delete(id); draftRevision.value += 1; }
  function edit(row: WorkflowRow, text: string): void {
    if (!scope.value) return;
    let own = drafts.get(scope.value); if (!own) { own = new Map(); drafts.set(scope.value, own); }
    own.set(row.version.id, { text, revision: own.get(row.version.id)?.revision ?? row.version.revision });
    draftRevision.value += 1;
  }
  async function discard(id: string): Promise<void> {
    if (draftStorage.supported.value) { await draftStorage.remove(id, true); }
    else dropDraft(id);
  }
  function draftIds(): string[] {
    void draftRevision.value;
    const h = status.value.hello; if (!ready.value || !h?.capabilities.includes('workflows.read')) return [];
    const allowed = h.resourceRestrictions?.['workflow'];
    return [...new Set([...(drafts.get(scope.value)?.keys() ?? []), ...draftStorage.visible.value.map((d) => d.id)])].filter((id) => allowed === undefined || allowed.includes(id));
  }
  function editRetainedDraft(id: string, text: string): void {
    const existing = draft(id); if (!existing || !draftIds().includes(id)) return;
    drafts.get(scope.value)!.set(id, { ...existing, text }); draftRevision.value += 1;
  }
  function rebaseDraft(row: WorkflowRow): void {
    const current = rows.value.find((r) => r.version.id === row.version.id && r.version.revision === row.version.revision);
    const own = draft(row.version.id);
    if (ready.value && current && own && draftIds().includes(row.version.id)) putDraft(row.version.id, { text: own.text, revision: row.version.revision });
  }
  const commandBlocked = computed(() => busy.value || commandSending.value || !!status.value.commandBusy || !!status.value.commandJournal?.pending || !!status.value.commandJournal?.unavailable);
  function journalMatches(): boolean {
    const s = status.value; const h = s.hello; const binding = s.commandJournal?.binding;
    return !!h && !!binding && binding.origin === s.origin && binding.hostId === h.hostId && binding.principalId === h.principalId && binding.sessionId === h.sessionId;
  }
  function canChange(operation: WorkflowOperation): boolean {
    const hello = status.value.hello;
    return ready.value && journalMatches() && !commandBlocked.value && !!hello?.capabilities.includes('workflows.read') &&
      hello.capabilities.includes(operation === 'disable' ? 'workflows.toggle' : 'workflows.edit') &&
      hello.features[operation === 'disable' ? 'workflowDisable' : 'workflowDelete'] === true;
  }
  function prepareCommand(row: WorkflowRow, operation: WorkflowOperation): CommandTarget | null {
    const s = status.value; const h = s.hello;
    if (!canChange(operation) || !h || !s.origin || !rows.value.some((r) => r.version.id === row.version.id && r.version.revision === row.version.revision)) return null;
    return { epoch: s.epoch, viewRevision: s.viewRevision, id: row.version.id, expectedRevision: row.version.revision, operation, origin: s.origin!, hostId: h.hostId, principalId: h.principalId, sessionId: h.sessionId };
  }
  function commandIsCurrent(target: CommandTarget): boolean {
    const s = status.value; const h = s.hello;
    return !!h && canChange(target.operation) && target.epoch === s.epoch && target.viewRevision === s.viewRevision && target.origin === s.origin &&
      target.hostId === h.hostId && target.principalId === h.principalId && target.sessionId === h.sessionId &&
      rows.value.some((r) => r.version.id === target.id && r.version.revision === target.expectedRevision);
  }
  const canResolve = computed(() => ready.value && journalMatches() && !busy.value && !commandSending.value && !status.value.commandBusy &&
    !!status.value.commandJournal?.pending && !status.value.commandJournal.unavailable && !!status.value.hello?.capabilities.includes('commands.read'));
  const canReview = computed(() => !busy.value && !commandSending.value && !status.value.commandBusy &&
    !!status.value.commandJournal?.pending && !status.value.commandJournal.unavailable);
  function prepareReview(): ReviewTarget | null {
    const journal = status.value.commandJournal;
    return canReview.value && journal?.pending ? { epoch: status.value.epoch, commandId: journal.pending.commandId, binding: JSON.stringify(journal.binding) } : null;
  }
  function reviewIsCurrent(target: ReviewTarget): boolean {
    const journal = status.value.commandJournal;
    return canReview.value && target.epoch === status.value.epoch && target.commandId === journal?.pending?.commandId && target.binding === JSON.stringify(journal?.binding);
  }
  async function commandCall(method: string, params: Record<string, unknown>): Promise<boolean> {
    if (commandSending.value || stopped) return false;
    const generation = selection;
    commandSending.value = true; error.value = '';
    let failureMessage = '';
    try { await control.call(method, params); return true; }
    catch (failure) { failureMessage = failure instanceof Error ? failure.message : 'Command outcome not confirmed'; return false; }
    finally {
      if (!stopped && generation === selection) { await refresh(); if (failureMessage && generation === selection) error.value = failureMessage; }
      commandSending.value = false;
    }
  }
  async function changeWorkflow(target: CommandTarget): Promise<boolean> {
    if (!commandIsCurrent(target)) return false;
    return commandCall('controller.workflow.command', { epoch: target.epoch, viewRevision: target.viewRevision, id: target.id, expectedRevision: target.expectedRevision, operation: target.operation });
  }
  async function selectPoints(id: string): Promise<void> {
    if (busy.value || !pointsAllowed.value || status.value.phase !== 'ready') return;
    const generation = selection; pointsError.value = ''; points.value = null; pointsLoaded.value = '';
    try { await control.call('controller.points.select', { epoch: status.value.epoch, uniqueId: id.trim().replace(/^@+/, '') || null }); }
    catch (failure) { if (generation === selection) pointsError.value = failure instanceof Error ? failure.message : 'Viewer unavailable'; }
    if (generation === selection) await refresh();
  }
  function pointsCanAdjust(): boolean {
    const h = status.value.hello;
    return ready.value && pointsAllowed.value && !!h?.connectionId && h.features['pointsAdjust'] === true && h.capabilities.includes('points.adjust') &&
      journalMatches() && !commandBlocked.value && pointsLoaded.value === pointsKey(status.value) && !!points.value?.currencyName && !points.value.loading && !points.value.errorCode;
  }
  function preparePoints(id: string, delta: number): PointsTarget | null {
    const s = status.value; const h = s.hello; const data = points.value;
    if (!pointsCanAdjust() || !h || !s.origin || !data?.currencyName || !Number.isFinite(delta) || delta === 0 || Math.abs(delta) > 1e9) return null;
    const known = data.rows?.some((r) => r.version.id === id) || data.selectedVersion?.id === id;
    if (!known) return null;
    return { epoch: s.epoch, connectionId: h.connectionId!, origin: s.origin, hostId: h.hostId, principalId: h.principalId, sessionId: h.sessionId,
      hostInstanceId: h.hostInstanceId, grantRevision: h.grantRevision, uniqueId: id, delta, currencyName: data.currencyName, creates: data.selectedVersion?.id === id && data.selectedVersion.revision === '0' };
  }
  function pointsIsCurrent(t: PointsTarget): boolean {
    const s = status.value; const h = s.hello;
    return !!h && ready.value && pointsAllowed.value && journalMatches() && !commandBlocked.value && !status.value.pointsLoading && !!points.value?.currencyName && points.value.selectedId === t.uniqueId && h.features['pointsAdjust'] === true && h.capabilities.includes('points.adjust') && t.epoch === s.epoch && t.connectionId === h.connectionId && t.origin === s.origin && t.hostId === h.hostId &&
      t.principalId === h.principalId && t.sessionId === h.sessionId && t.hostInstanceId === h.hostInstanceId && t.grantRevision === h.grantRevision &&
      t.currencyName === points.value?.currencyName && (h.resourceRestrictions?.['viewer'] === undefined || h.resourceRestrictions['viewer'].includes(t.uniqueId));
  }
  async function adjustPoints(t: PointsTarget): Promise<boolean> {
    if (!pointsIsCurrent(t)) return false;
    return commandCall('controller.points.command', { epoch: t.epoch, connectionId: t.connectionId, uniqueId: t.uniqueId, delta: t.delta, currencyName: t.currencyName });
  }
  async function resolveCommand(): Promise<boolean> {
    if (!canResolve.value) return false;
    return commandCall('controller.command.resolve', { epoch: status.value.epoch, commandId: status.value.commandJournal!.pending!.commandId });
  }
  async function reviewUnknown(target: ReviewTarget): Promise<boolean> {
    if (!reviewIsCurrent(target)) return false;
    return commandCall('controller.command.review', { epoch: target.epoch, commandId: target.commandId, acknowledgeUnknown: true });
  }
  async function start(profile?: string | null): Promise<void> {
    stopped = false; control.attach(); unsubscribe = control.onTopic('controller.changed', () => { void refresh(); });
    timer = setInterval(() => { void refresh(); }, 1000);
    try { await inventory(); if (profile) await connect(profile); else await refresh(); }
    catch (failure) { error.value = failure instanceof Error ? failure.message : 'Profiles unavailable'; }
  }
  const draftStorage = createDraftPersistence(control, { status, ready, scope, generation: () => selection, stopped: () => stopped, read: draft, put: putDraft, drop: dropDraft });
  const automation = createAutomationModel(control, { status, generation: () => selection, stopped: () => stopped, retry: () => { dirty = true; }, blocked: () => commandBlocked.value, journalMatches, send: commandCall });
  const proposals = createProposalModel(control, { status, blocked: () => commandBlocked.value, journalMatches, send: commandCall });
  const live = createLiveStatusModel(control, { status, generation: () => selection, stopped: () => stopped });
  const liveFeed = createLiveFeedModel(control, { status, generation: () => selection, stopped: () => stopped });
  const competitions = createCompetitionModel(control, { status, generation: () => selection, stopped: () => stopped, blocked: () => commandBlocked.value, journalMatches, send: commandCall });
  function stop(): void { proposals.stop(); automation.clear(); live.clear(); liveFeed.clear(); competitions.clear(); stopped = true; if (timer) clearInterval(timer); unsubscribe?.(); control.detach(); rows.value = []; points.value = null; pointsLoaded.value = ''; draftStorage.stop(); status.value = idle(); loaded.value = ''; scope.value = ''; drafts.clear(); }
  return { proposals, automation, live, liveFeed, competitions, status, profiles, rows, points, pointsError, pointsAllowed, selectPoints, pointsCanAdjust, preparePoints, pointsIsCurrent, adjustPoints, error, busy, ready, commandSending, commandBlocked, canChange, canResolve, canReview, prepareForget, profileForgetCurrent, forgetProfile,
    prepareCommand, commandIsCurrent, changeWorkflow, resolveCommand, prepareReview, reviewIsCurrent, reviewUnknown,
    inventory, refresh, start, stop, connect, disconnect, draft, draftIds, editRetainedDraft, edit, discard, rebaseDraft, draftStorage };
}
