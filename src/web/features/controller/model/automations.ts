import { computed, ref, type Ref } from 'vue';
import type { ControlClient, ResourceVersion } from '@/shared/api/control-client.ts';
import type { ControllerStatus } from './controller.ts';

export type EventOperation = 'disable-event' | 'delete-event' | 'enable-event' | 'enable-action';
export interface AutomationSummary { kind: 'event' | 'action'; id: string; name: string; enabled: boolean; definitionRedacted: true }
export interface AutomationRow { summary: AutomationSummary; version: ResourceVersion }
export interface AutomationPage { epoch: string; automationRevision: string; rows: AutomationRow[]; next: number | null }
export interface AutomationTarget {
  epoch: string; automationRevision: string; connectionId: string; hostInstanceId: string; grantRevision: string;
  origin: string; profile: string; hostId: string; principalId: string; sessionId: string;
  id: string; name: string; expectedRevision: string; operation: EventOperation;
}
interface Dependencies {
  status: Ref<ControllerStatus>; generation: () => number; stopped: () => boolean; retry: () => void;
  blocked: () => boolean; journalMatches: () => boolean; send: (method: string, params: Record<string, unknown>) => Promise<boolean>;
}
const encoder = new TextEncoder();
const canonicalRevision = (value: unknown): value is string => typeof value === 'string' && value.length <= 19 && /^(0|[1-9]\d*)$/.test(value) && BigInt(value) <= 9223372036854775807n;
const record = (value: unknown): value is Record<string, unknown> => value !== null && typeof value === 'object' && !Array.isArray(value);
function binding(s: ControllerStatus): string {
  const h = s.hello;
  return JSON.stringify([s.epoch, s.profile, s.origin, h?.connectionId, h?.hostId, h?.hostInstanceId, h?.principalId, h?.sessionId, h?.grantRevision, h?.resourceRestrictions]);
}
function readable(s: ControllerStatus): boolean {
  return s.phase === 'ready' && s.hello?.features['automationSummary'] === true && s.hello.capabilities.includes('automation.read') &&
    !Object.prototype.hasOwnProperty.call(s.hello.resourceRestrictions ?? {}, 'live');
}
function permitted(s: ControllerStatus, family: string, id: string): boolean {
  const ids = s.hello?.resourceRestrictions?.['automation'];
  return ids === undefined || ids.includes(`${family}:${id}`);
}
function row(input: unknown, status: ControllerStatus): AutomationRow {
  if (!record(input) || !record(input['summary']) || !record(input['version'])) throw new Error('Invalid automation status');
  const s = input['summary']; const v = input['version'];
  if (Object.keys(s).length !== 5 || Object.keys(s).some((k) => !['kind', 'id', 'name', 'enabled', 'definitionRedacted'].includes(k)) ||
    (s['kind'] !== 'event' && s['kind'] !== 'action') || typeof s['id'] !== 'string' || !s['id'] || s['id'].trim() !== s['id'] || encoder.encode(s['id']).length > 128 || /[\p{Cc}*]/u.test(s['id']) ||
    typeof s['name'] !== 'string' || [...s['name']].length > 160 || typeof s['enabled'] !== 'boolean' || s['definitionRedacted'] !== true ||
    v['kind'] !== 'automation' || v['scope'] !== s['kind'] || v['id'] !== s['id'] || !canonicalRevision(v['revision']) || v['revision'] === '0' || !permitted(status, String(s['kind']), s['id'])) throw new Error('Invalid automation status');
  return { summary: { kind: s['kind'] as 'event' | 'action', id: s['id'], name: s['name'], enabled: s['enabled'], definitionRedacted: true },
    version: { kind: 'automation', scope: String(s['kind']), id: s['id'], revision: v['revision'] } };
}

/** Independent closed status projection. It never enters the definition editor. */
export function createAutomationModel(control: ControlClient, deps: Dependencies) {
  const rows = ref<AutomationRow[]>([]); const error = ref(''); const loaded = ref(''); let token = 0;
  const key = (s: ControllerStatus) => `${binding(s)}:${s.automationRevision ?? ''}`;
  const identity = computed(() => binding(deps.status.value));
  const allowed = computed(() => readable(deps.status.value));
  const ready = computed(() => allowed.value && deps.status.value.automationRevision !== undefined && loaded.value === key(deps.status.value));
  function clear(): void { token += 1; rows.value = []; loaded.value = ''; error.value = ''; }
  async function refresh(current: ControllerStatus, generation: number): Promise<void> {
    if (!readable(current) || current.automationRevision === undefined) { clear(); return; }
    if (!canonicalRevision(current.automationRevision)) { clear(); error.value = 'Invalid automation view revision'; return; }
    if (loaded.value === key(current)) return;
    clear(); const request = token; const nextRows: AutomationRow[] = []; const seen = new Set<string>(); const positions = new Set<number>(); let bytes = 0;
    try {
      let offset: number | null = 0;
      while (offset !== null) {
        if (!Number.isSafeInteger(offset) || offset < 0 || positions.has(offset) || positions.size >= 4096) throw new Error('Invalid automation pagination');
        positions.add(offset);
        const page: AutomationPage = await control.call<AutomationPage>('controller.automations', { epoch: current.epoch, automationRevision: current.automationRevision, offset });
        if (deps.stopped() || request !== token || generation !== deps.generation()) return;
        if (page.epoch !== current.epoch || page.automationRevision !== current.automationRevision || !Array.isArray(page.rows) ||
          (page.next !== null && (!Number.isSafeInteger(page.next) || page.next <= offset || !page.rows.length))) throw new Error('Automation view changed');
        bytes += encoder.encode(JSON.stringify(page)).length;
        if (bytes > 4 * 1024 * 1024) throw new Error('Automation status exceeds its display budget');
        for (const item of page.rows) {
          const validated = row(item, current); const id = `${validated.summary.kind}:${validated.summary.id}`;
          if (seen.has(id)) throw new Error('Duplicate automation status');
          seen.add(id); nextRows.push(validated);
        }
        offset = page.next;
      }
      const after = await control.call<ControllerStatus>('controller.status');
      if (deps.stopped() || request !== token || generation !== deps.generation()) return;
      if (!readable(after) || key(after) !== key(current) || key(deps.status.value) !== key(current)) { clear(); deps.retry(); return; }
      rows.value = nextRows; loaded.value = key(current); error.value = '';
    } catch (failure) {
      if (deps.stopped() || request !== token || generation !== deps.generation()) return;
      rows.value = []; loaded.value = ''; error.value = failure instanceof Error ? failure.message : 'Automation status unavailable';
    }
  }
  function canChange(item: AutomationRow, operation: EventOperation): boolean {
    const s = deps.status.value; const h = s.hello;
    const activation = operation === 'enable-event' || operation === 'enable-action';
    const family = operation === 'enable-action' ? 'action' : 'event';
    return ready.value && deps.journalMatches() && !deps.blocked() && !!h?.connectionId && !!h.grantRevision &&
      h.features[activation ? 'automationReusableActivation' : 'automationEventDeactivate'] === true &&
      h.capabilities.includes(operation === 'delete-event' ? 'automation.edit' : 'automation.toggle') &&
      item.summary.kind === family && (activation ? !item.summary.enabled : operation !== 'disable-event' || item.summary.enabled) && permitted(s, family, item.summary.id) &&
      rows.value.some((r) => r.summary.kind === family && r.summary.id === item.summary.id && r.version.revision === item.version.revision && r.summary.name === item.summary.name && r.summary.enabled === item.summary.enabled);
  }
  function prepare(item: AutomationRow, operation: EventOperation): AutomationTarget | null {
    const s = deps.status.value; const h = s.hello;
    if (!canChange(item, operation) || !h || !s.origin || !s.profile) return null;
    return { epoch: s.epoch, automationRevision: s.automationRevision!, connectionId: h.connectionId!, grantRevision: h.grantRevision!, hostInstanceId: h.hostInstanceId,
      origin: s.origin, profile: s.profile, hostId: h.hostId, principalId: h.principalId, sessionId: h.sessionId, id: item.summary.id, name: item.summary.name, expectedRevision: item.version.revision, operation };
  }
  function current(target: AutomationTarget): boolean {
    const s = deps.status.value; const h = s.hello;
    const family = target.operation === 'enable-action' ? 'action' : 'event';
    const item = rows.value.find((r) => r.summary.kind === family && r.summary.id === target.id && r.version.revision === target.expectedRevision && r.summary.name === target.name);
    return !!h && !!item && canChange(item, target.operation) && target.epoch === s.epoch && target.profile === s.profile && target.automationRevision === s.automationRevision &&
      target.connectionId === h.connectionId && target.grantRevision === h.grantRevision && target.origin === s.origin && target.hostId === h.hostId &&
      target.hostInstanceId === h.hostInstanceId && target.principalId === h.principalId && target.sessionId === h.sessionId;
  }
  async function change(target: AutomationTarget): Promise<boolean> {
    if (!current(target)) return false;
    return deps.send('controller.automation.command', { epoch: target.epoch, automationRevision: target.automationRevision, connectionId: target.connectionId,
      hostInstanceId: target.hostInstanceId, grantRevision: target.grantRevision, id: target.id, expectedRevision: target.expectedRevision, operation: target.operation });
  }
  return { identity, rows, error, allowed, ready, clear, refresh, canChange, prepare, current, change };
}
export type AutomationModel = ReturnType<typeof createAutomationModel>;
