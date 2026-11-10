import { computed, ref, type Ref } from 'vue';
import type { ControlClient } from '@/shared/api/control-client.ts';
import type { ControllerStatus } from './controller.ts';

export interface LiveStatus { connected: boolean; uniqueId: string | null; native: boolean }
interface Dependencies { status: Ref<ControllerStatus>; generation: () => number; stopped: () => boolean }
const binding = (s: ControllerStatus) => JSON.stringify([s.epoch, s.profile, s.origin, s.hello?.connectionId,
  s.hello?.hostId, s.hello?.hostInstanceId, s.hello?.principalId, s.hello?.sessionId,
  s.hello?.grantRevision, s.hello?.capabilities, s.hello?.resourceRestrictions]);
const readable = (s: ControllerStatus) => s.phase === 'ready' && !!s.hello?.connectionId && s.hello.capabilities.includes('live.read');
function decode(value: unknown, status: ControllerStatus): LiveStatus {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid LIVE status');
  const result = value as Record<string, unknown>;
  if (Object.keys(result).length !== 5 || Object.keys(result).some((k) => !['epoch','connectionId','connected','uniqueId','native'].includes(k)) ||
    result['epoch'] !== status.epoch || result['connectionId'] !== status.hello?.connectionId || typeof result['connected'] !== 'boolean' ||
    typeof result['native'] !== 'boolean' || !(result['uniqueId'] === null || (typeof result['uniqueId'] === 'string' &&
      result['uniqueId'].length > 0 && new TextEncoder().encode(result['uniqueId']).length <= 256 && !/\p{Cc}/u.test(result['uniqueId'])))) throw new Error('Invalid LIVE status');
  const allowed = status.hello?.resourceRestrictions?.['live'];
  if (allowed !== undefined && (typeof result['uniqueId'] !== 'string' || !allowed.includes(result['uniqueId']))) throw new Error('LIVE creator is not permitted');
  return { connected: result['connected'], uniqueId: result['uniqueId'] as string | null, native: result['native'] };
}

/** Read-only and session-bound; no remote command is ever sent here. */
export function createLiveStatusModel(control: ControlClient, deps: Dependencies) {
  const data = ref<LiveStatus | null>(null); const error = ref('');
  const allowed = computed(() => readable(deps.status.value));
  let token = 0; let inFlight = false; let nextPoll = 0; let loadedBinding = '';
  function clear(): void { token += 1; inFlight = false; nextPoll = 0; loadedBinding = ''; data.value = null; error.value = ''; }
  async function refresh(current: ControllerStatus, generation: number): Promise<void> {
    if (!readable(current)) { clear(); return; }
    const captured = binding(current);
    if (loadedBinding && loadedBinding !== captured) clear();
    if (inFlight || Date.now() < nextPoll) return;
    const request = ++token; inFlight = true; loadedBinding = captured; nextPoll = Date.now() + 2000;
    try {
      const response = await control.call<unknown>('controller.live.status', { epoch: current.epoch, connectionId: current.hello!.connectionId });
      const value = decode(response, current);
      const after = await control.call<ControllerStatus>('controller.status');
      if (deps.stopped() || request !== token || generation !== deps.generation()) return;
      if (!readable(after) || binding(after) !== captured || binding(deps.status.value) !== captured) { clear(); return; }
      data.value = value; error.value = '';
    } catch {
      if (deps.stopped() || request !== token || generation !== deps.generation()) return;
      data.value = null; error.value = 'LIVE status unavailable';
    } finally { if (request === token) inFlight = false; }
  }
  return { data, error, allowed, clear, refresh };
}
export type LiveStatusModel = ReturnType<typeof createLiveStatusModel>;
