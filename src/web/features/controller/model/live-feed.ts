import { computed, ref, type Ref } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import type { ControllerStatus } from './controller.ts';

export interface LiveFeedEvent { cursor: string; kind: string; author: string; authorLookupId: string | null; nickname: string | null; text: string }
interface Dependencies { status: Ref<ControllerStatus>; generation: () => number; stopped: () => boolean }
interface Page { nextCursor: string; latestCursor: string; gap: boolean; truncated: boolean; events: LiveFeedEvent[]; streamId?: string | null }
const kinds = new Set(['chat', 'gift', 'like', 'member', 'social']);
const maxCursor = 18446744073709551615n;
const cursor = (v: unknown): v is string => typeof v === 'string' && /^(0|[1-9]\d{0,19})$/.test(v) && BigInt(v) <= maxCursor;
const binding = (s: ControllerStatus) => JSON.stringify([s.epoch, s.profile, s.origin, s.hello?.connectionId,
  s.hello?.hostId, s.hello?.hostInstanceId, s.hello?.principalId, s.hello?.sessionId,
  s.hello?.grantRevision, s.hello?.capabilities, s.hello?.resourceRestrictions]);
const readable = (s: ControllerStatus) => s.phase === 'ready' && !!s.hello?.connectionId &&
  s.hello.capabilities.includes('live.read') && s.hello.features['liveFeedSupported'] !== false && (s.hello.resourceRestrictions?.['live'] === undefined ||
    (s.hello.features['creatorFeed'] === true && s.hello.resourceRestrictions['live'].length > 0));

function decode(value: unknown, status: ControllerStatus, after: string, creator: string | null, previousStream: string | null): Page {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid LIVE feed');
  const p = value as Record<string, unknown>;
  const keys = creator === null ? ['epoch','connectionId','oldestCursor','latestCursor','nextCursor','gap','truncated','events'] : ['epoch','connectionId','creatorId','streamId','oldestCursor','latestCursor','nextCursor','gap','truncated','events'];
  if (Object.keys(p).length !== keys.length || Object.keys(p).some((key) => !keys.includes(key)) ||
    p['epoch'] !== status.epoch || p['connectionId'] !== status.hello?.connectionId ||
    (creator !== null && (p['creatorId'] !== creator || !(p['streamId'] === null || (typeof p['streamId'] === 'string' && /^[0-9a-f]{32}$/.test(p['streamId']))))) ||
    !cursor(p['oldestCursor']) || !cursor(p['latestCursor']) || !cursor(p['nextCursor']) ||
    typeof p['gap'] !== 'boolean' || typeof p['truncated'] !== 'boolean' || !Array.isArray(p['events']) || p['events'].length > 64) throw new Error('Invalid LIVE feed');
  if (creator !== null && p['streamId'] === null && (p['oldestCursor'] !== '0' || p['latestCursor'] !== '0' || p['nextCursor'] !== '0' || p['events'].length !== 0)) throw new Error('Invalid LIVE feed stream');
  const restarted = BigInt(after) > BigInt(p['latestCursor']) || (creator !== null && previousStream !== null && previousStream !== p['streamId']);
  if (restarted && !p['gap']) throw new Error('Invalid LIVE feed reset');
  let previous = restarted ? 0n : BigInt(after); let first = true;
  const events: LiveFeedEvent[] = p['events'].map((value: unknown) => {
    if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid LIVE event');
    const e = value as Record<string, unknown>;
    if (Object.keys(e).length !== 6 || Object.keys(e).some((key) => !['cursor','kind','author','authorLookupId','nickname','text'].includes(key)) ||
      !cursor(e['cursor']) || BigInt(e['cursor']) <= previous || BigInt(e['cursor']) > BigInt(p['latestCursor'] as string) ||
      (first && previous === 0n && e['cursor'] !== p['oldestCursor']) ||
      ((!first || (previous !== 0n && !p['gap'])) && BigInt(e['cursor']) !== previous + 1n) ||
      typeof e['kind'] !== 'string' || !kinds.has(e['kind']) || typeof e['author'] !== 'string' || [...e['author']].length > 128 ||
      (creator !== null && [...e['author']].length > 64) ||
      !(e['authorLookupId'] === null || (typeof e['authorLookupId'] === 'string' && e['authorLookupId'] === e['author'] &&
        e['authorLookupId'] !== 'viewer' && e['authorLookupId'].length > 0 && e['authorLookupId'].trim() === e['authorLookupId'] &&
        !e['authorLookupId'].startsWith('@') && !/[\p{Cc}*]/u.test(e['authorLookupId']) && new TextEncoder().encode(e['authorLookupId']).length <= 128)) ||
      !(e['nickname'] === null || (typeof e['nickname'] === 'string' && [...e['nickname']].length <= (creator === null ? 128 : 64))) ||
      typeof e['text'] !== 'string' || [...e['text']].length > (creator === null ? 2048 : 128)) throw new Error('Invalid LIVE event');
    previous = BigInt(e['cursor']); first = false;
    return { cursor: e['cursor'], kind: e['kind'], author: e['author'], authorLookupId: e['authorLookupId'] as string | null, nickname: e['nickname'] as string | null, text: e['text'] };
  });
  const exhausted = creator !== null && p['gap'] && p['oldestCursor'] === '0' && events.length === 0 && p['nextCursor'] === p['latestCursor'];
  if ((creator === null && restarted ? (!p['gap'] || events.length !== 0 || p['nextCursor'] !== '0') : BigInt(p['nextCursor']) !== previous && !exhausted) ||
    (creator === null && BigInt(after) < BigInt(p['latestCursor']) && events.length === 0) || p['truncated'] !== (creator === null && restarted ? false : BigInt(p['nextCursor']) < BigInt(p['latestCursor']))) throw new Error('Invalid LIVE feed cursor');
  return { nextCursor: p['nextCursor'], latestCursor: p['latestCursor'], gap: p['gap'], truncated: p['truncated'], events, ...(creator === null ? {} : { streamId: p['streamId'] as string | null }) };
}

/** Bounded read-only LIVE activity; no commands or durable replay. */
export function createLiveFeedModel(control: ControlClient, deps: Dependencies) {
  const events = ref<LiveFeedEvent[]>([]); const gap = ref(false); const error = ref(''); const scopeRevision = ref(0);
  const allowed = computed(() => readable(deps.status.value));
  const creatorOptions = computed(() => [...(deps.status.value.hello?.resourceRestrictions?.['live'] ?? [])].sort());
  const selectedCreator = ref<string | null>(null);
  let token = 0; let inFlight = false; let nextPoll = 0; let loadedBinding = ''; let lastCursor = '0'; let lastStream: string | null = null;
  function clear(): void { token += 1; scopeRevision.value += 1; inFlight = false; nextPoll = 0; loadedBinding = ''; lastCursor = '0'; lastStream = null; events.value = []; gap.value = false; error.value = ''; }
  function selectCreator(value: string): void { if (creatorOptions.value.includes(value) && value !== selectedCreator.value) { clear(); selectedCreator.value = value; } }
  async function refresh(current: ControllerStatus, generation: number): Promise<void> {
    if (!readable(current)) { clear(); return; }
    const restricted = current.hello?.resourceRestrictions?.['live'];
    if (restricted && (!selectedCreator.value || !restricted.includes(selectedCreator.value))) { clear(); selectedCreator.value = [...restricted].sort()[0] ?? null; }
    if (!restricted) selectedCreator.value = null;
    const creator = restricted ? selectedCreator.value : null;
    const captured = JSON.stringify([binding(current), creator]);
    if (loadedBinding && loadedBinding !== captured) clear();
    if (inFlight || Date.now() < nextPoll) return;
    const request = ++token; inFlight = true; loadedBinding = captured; nextPoll = Date.now() + 2000;
    const after = lastCursor;
    try {
      const stream = lastStream;
      const response = await control.call<unknown>('controller.live.feed', { epoch: current.epoch, connectionId: current.hello!.connectionId, after, ...(creator === null ? {} : { creatorId: creator, streamId: stream }) });
      const page = decode(response, current, after, creator, stream);
      const checked = await control.call<ControllerStatus>('controller.status');
      if (deps.stopped() || request !== token || generation !== deps.generation()) return;
      if (!readable(checked) || JSON.stringify([binding(checked), creator]) !== captured || JSON.stringify([binding(deps.status.value), selectedCreator.value]) !== captured) { clear(); return; }
      if (BigInt(after) > BigInt(page.latestCursor) || (creator !== null && stream !== null && stream !== page.streamId)) events.value = [];
      events.value = [...events.value, ...page.events].slice(-100);
      gap.value ||= page.gap; lastCursor = page.nextCursor; lastStream = page.streamId ?? null; error.value = '';
      if (page.truncated) nextPoll = Date.now();
    } catch {
      if (deps.stopped() || request !== token || generation !== deps.generation()) return;
      events.value = []; gap.value = true; lastCursor = '0'; error.value = 'LIVE feed unavailable';
    } finally { if (request === token) inFlight = false; }
  }
  return { events, gap, error, allowed, scopeRevision, creatorOptions, selectedCreator, selectCreator, clear, refresh };
}
export type LiveFeedModel = ReturnType<typeof createLiveFeedModel>;
