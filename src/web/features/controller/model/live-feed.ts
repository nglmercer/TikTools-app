import { computed, ref, type Ref } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import type { ControllerStatus } from './controller.ts';

export interface LiveFeedEvent { cursor: string; kind: string; author: string; nickname: string | null; text: string }
interface Dependencies { status: Ref<ControllerStatus>; generation: () => number; stopped: () => boolean }
interface Page { nextCursor: string; latestCursor: string; gap: boolean; truncated: boolean; events: LiveFeedEvent[] }
const kinds = new Set(['chat', 'gift', 'like', 'member', 'social']);
const maxCursor = 18446744073709551615n;
const cursor = (v: unknown): v is string => typeof v === 'string' && /^(0|[1-9]\d{0,19})$/.test(v) && BigInt(v) <= maxCursor;
const binding = (s: ControllerStatus) => JSON.stringify([s.epoch, s.profile, s.origin, s.hello?.connectionId,
  s.hello?.hostId, s.hello?.hostInstanceId, s.hello?.principalId, s.hello?.sessionId,
  s.hello?.grantRevision, s.hello?.capabilities, s.hello?.resourceRestrictions]);
const readable = (s: ControllerStatus) => s.phase === 'ready' && !!s.hello?.connectionId &&
  s.hello.capabilities.includes('live.read') && s.hello.resourceRestrictions?.['live'] === undefined;

function decode(value: unknown, status: ControllerStatus, after: string): Page {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid LIVE feed');
  const p = value as Record<string, unknown>;
  if (Object.keys(p).length !== 8 || Object.keys(p).some((key) => !['epoch','connectionId','oldestCursor','latestCursor','nextCursor','gap','truncated','events'].includes(key)) ||
    p['epoch'] !== status.epoch || p['connectionId'] !== status.hello?.connectionId ||
    !cursor(p['oldestCursor']) || !cursor(p['latestCursor']) || !cursor(p['nextCursor']) ||
    typeof p['gap'] !== 'boolean' || typeof p['truncated'] !== 'boolean' || !Array.isArray(p['events']) || p['events'].length > 64) throw new Error('Invalid LIVE feed');
  let previous = BigInt(after); let first = true;
  const events: LiveFeedEvent[] = p['events'].map((value: unknown) => {
    if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid LIVE event');
    const e = value as Record<string, unknown>;
    if (Object.keys(e).length !== 5 || Object.keys(e).some((key) => !['cursor','kind','author','nickname','text'].includes(key)) ||
      !cursor(e['cursor']) || BigInt(e['cursor']) <= previous || BigInt(e['cursor']) > BigInt(p['latestCursor'] as string) ||
      (first && after === '0' && e['cursor'] !== p['oldestCursor']) ||
      ((!first || (after !== '0' && !p['gap'])) && BigInt(e['cursor']) !== previous + 1n) ||
      typeof e['kind'] !== 'string' || !kinds.has(e['kind']) || typeof e['author'] !== 'string' || [...e['author']].length > 128 ||
      !(e['nickname'] === null || (typeof e['nickname'] === 'string' && [...e['nickname']].length <= 128)) ||
      typeof e['text'] !== 'string' || [...e['text']].length > 2048) throw new Error('Invalid LIVE event');
    previous = BigInt(e['cursor']); first = false;
    return { cursor: e['cursor'], kind: e['kind'], author: e['author'], nickname: e['nickname'] as string | null, text: e['text'] };
  });
  const restarted = BigInt(after) > BigInt(p['latestCursor']);
  if ((restarted ? (!p['gap'] || events.length !== 0 || p['nextCursor'] !== '0') : BigInt(p['nextCursor']) !== previous) ||
    (BigInt(after) < BigInt(p['latestCursor']) && events.length === 0) || p['truncated'] !== (previous < BigInt(p['latestCursor']))) throw new Error('Invalid LIVE feed cursor');
  return { nextCursor: p['nextCursor'], latestCursor: p['latestCursor'], gap: p['gap'], truncated: p['truncated'], events };
}

/** Bounded read-only LIVE activity; no commands or durable replay. */
export function createLiveFeedModel(control: ControlClient, deps: Dependencies) {
  const events = ref<LiveFeedEvent[]>([]); const gap = ref(false); const error = ref(''); const scopeRevision = ref(0);
  const allowed = computed(() => readable(deps.status.value));
  let token = 0; let inFlight = false; let nextPoll = 0; let loadedBinding = ''; let lastCursor = '0';
  function clear(): void { token += 1; scopeRevision.value += 1; inFlight = false; nextPoll = 0; loadedBinding = ''; lastCursor = '0'; events.value = []; gap.value = false; error.value = ''; }
  async function refresh(current: ControllerStatus, generation: number): Promise<void> {
    if (!readable(current)) { clear(); return; }
    const captured = binding(current);
    if (loadedBinding && loadedBinding !== captured) clear();
    if (inFlight || Date.now() < nextPoll) return;
    const request = ++token; inFlight = true; loadedBinding = captured; nextPoll = Date.now() + 2000;
    const after = lastCursor;
    try {
      const response = await control.call<unknown>('controller.live.feed', { epoch: current.epoch, connectionId: current.hello!.connectionId, after });
      const page = decode(response, current, after);
      const checked = await control.call<ControllerStatus>('controller.status');
      if (deps.stopped() || request !== token || generation !== deps.generation()) return;
      if (!readable(checked) || binding(checked) !== captured || binding(deps.status.value) !== captured) { clear(); return; }
      if (BigInt(after) > BigInt(page.latestCursor)) events.value = [];
      events.value = [...events.value, ...page.events].slice(-100);
      gap.value ||= page.gap; lastCursor = page.nextCursor; error.value = '';
      if (page.truncated) nextPoll = Date.now();
    } catch {
      if (deps.stopped() || request !== token || generation !== deps.generation()) return;
      events.value = []; gap.value = true; lastCursor = '0'; error.value = 'LIVE feed unavailable';
    } finally { if (request === token) inFlight = false; }
  }
  return { events, gap, error, allowed, scopeRevision, clear, refresh };
}
export type LiveFeedModel = ReturnType<typeof createLiveFeedModel>;
