import { computed, ref, type Ref } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import type { ControllerStatus } from './controller.ts';

export interface CompetitionSummary {
  id: string; name: string; kind: string; lifecycle: string;
  generation: string; deadlineMs: string | null; remainingMs: string | null;
  revision?: string;
}
interface Dependencies { status: Ref<ControllerStatus>; generation: () => number; stopped: () => boolean }
interface Page { competitions: CompetitionSummary[]; nextAfter: string | null }
const kinds = new Set(['poll', 'weighted_poll', 'leaderboard_challenge', 'team_battle', 'random_draw', 'weighted_draw', 'roulette']);
const states = new Set(['draft', 'active', 'paused', 'finalizing', 'finished', 'cancelled', 'failed']);
const max = 18446744073709551615n;
const decimal = (value: unknown): value is string => typeof value === 'string' && /^(0|[1-9]\d{0,19})$/.test(value) && BigInt(value) <= max;
const text = (value: unknown, bytes: number): value is string => typeof value === 'string' && value.length > 0 &&
  new TextEncoder().encode(value).length <= bytes && !/\p{Cc}/u.test(value);
function byteCompare(a: string, b: string): number {
  const left = new TextEncoder().encode(a); const right = new TextEncoder().encode(b);
  for (let i = 0; i < Math.min(left.length, right.length); i += 1) {
    if (left[i] !== right[i]) return left[i]! - right[i]!;
  }
  return left.length - right.length;
}
const binding = (s: ControllerStatus) => JSON.stringify([s.epoch, s.profile, s.origin, s.hello?.connectionId,
  s.hello?.hostId, s.hello?.hostInstanceId, s.hello?.principalId, s.hello?.sessionId,
  s.hello?.grantRevision, s.hello?.capabilities, s.hello?.features, s.hello?.resourceRestrictions]);
const readable = (s: ControllerStatus) => s.phase === 'ready' && !!s.hello?.connectionId &&
  s.hello.features['competitionSummary'] === true && s.hello.capabilities.includes('competitions.read') &&
  !Object.prototype.hasOwnProperty.call(s.hello.resourceRestrictions ?? {}, 'live');

function decode(value: unknown, s: ControllerStatus, after: string | null): Page {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid competition page');
  const page = value as Record<string, unknown>;
  if (Object.keys(page).length !== 4 || Object.keys(page).some((k) => !['epoch','connectionId','competitions','nextAfter'].includes(k)) ||
    page['epoch'] !== s.epoch || page['connectionId'] !== s.hello?.connectionId || !Array.isArray(page['competitions']) || page['competitions'].length > 20 ||
    !(page['nextAfter'] === null || text(page['nextAfter'], 128))) throw new Error('Invalid competition page');
  let previous = after;
  const rows: CompetitionSummary[] = page['competitions'].map((value: unknown) => {
    if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid competition');
    const row = value as Record<string, unknown>;
    if ((Object.keys(row).length !== 7 && Object.keys(row).length !== 8) || Object.keys(row).some((k) => !['id','name','kind','lifecycle','generation','revision','deadlineMs','remainingMs'].includes(k)) ||
      !text(row['id'], 128) || !text(row['name'], 256) || !row['name'].trim() ||
      (previous !== null && byteCompare(row['id'], previous) <= 0) || typeof row['kind'] !== 'string' || !kinds.has(row['kind']) ||
      typeof row['lifecycle'] !== 'string' || !states.has(row['lifecycle']) || !decimal(row['generation']) ||
      (s.hello?.features['competitionSummaryVersions'] === true && row['revision'] === undefined) ||
      (row['revision'] !== undefined && (!decimal(row['revision']) || row['revision'] === '0' || BigInt(row['revision']) > 9223372036854775807n)) ||
      !(row['deadlineMs'] === null || decimal(row['deadlineMs'])) || !(row['remainingMs'] === null || decimal(row['remainingMs'])) ||
      (s.hello?.resourceRestrictions?.['competition'] !== undefined && !s.hello.resourceRestrictions['competition'].includes(row['id']))) throw new Error('Invalid competition');
    previous = row['id'];
    return row as unknown as CompetitionSummary;
  });
  if (page['nextAfter'] !== null && page['nextAfter'] !== rows.at(-1)?.id) throw new Error('Invalid competition cursor');
  return { competitions: rows, nextAfter: page['nextAfter'] as string | null };
}

/** Read-only live keyset pages; refresh never issues a competition command. */
export function createCompetitionModel(control: ControlClient, deps: Dependencies) {
  const rows = ref<CompetitionSummary[]>([]); const error = ref(''); const loading = ref(false);
  const pageIndex = ref(0); const allowed = computed(() => readable(deps.status.value));
  const nextAfter = ref<string | null>(null); const cursors = ref<(string | null)[]>([null]);
  let token = 0; let nextPoll = 0; let loadedBinding = '';
  function clear(): void { token += 1; rows.value = []; error.value = ''; loading.value = false; pageIndex.value = 0; nextAfter.value = null; cursors.value = [null]; nextPoll = 0; loadedBinding = ''; }
  async function refresh(current: ControllerStatus, generation: number): Promise<void> {
    if (!readable(current)) { clear(); return; }
    const captured = binding(current);
    if (loadedBinding && loadedBinding !== captured) clear();
    if (loading.value || performance.now() < nextPoll) return;
    const request = ++token; const after = cursors.value[pageIndex.value] ?? null;
    loading.value = true; loadedBinding = captured; nextPoll = performance.now() + 2000;
    try {
      const response = await control.call<unknown>('controller.competitions', { epoch: current.epoch, connectionId: current.hello!.connectionId, after });
      const page = decode(response, current, after);
      const checked = await control.call<ControllerStatus>('controller.status');
      if (deps.stopped() || request !== token || generation !== deps.generation()) return;
      if (!readable(checked) || binding(checked) !== captured || binding(deps.status.value) !== captured) { clear(); return; }
      rows.value = page.competitions; nextAfter.value = page.nextAfter; error.value = '';
    } catch {
      if (deps.stopped() || request !== token || generation !== deps.generation()) return;
      rows.value = []; nextAfter.value = null; error.value = 'Competition status unavailable';
    } finally { if (request === token) loading.value = false; }
  }
  async function next(): Promise<void> {
    if (loading.value || nextAfter.value === null) return;
    cursors.value = [...cursors.value.slice(0, pageIndex.value + 1), nextAfter.value];
    pageIndex.value += 1; rows.value = []; nextAfter.value = null; nextPoll = 0;
    await refresh(deps.status.value, deps.generation());
  }
  async function previous(): Promise<void> {
    if (loading.value || pageIndex.value === 0) return;
    pageIndex.value -= 1; rows.value = []; nextAfter.value = null; nextPoll = 0;
    await refresh(deps.status.value, deps.generation());
  }
  return { rows, error, loading, pageIndex, nextAfter, allowed, clear, refresh, next, previous };
}
export type CompetitionModel = ReturnType<typeof createCompetitionModel>;
