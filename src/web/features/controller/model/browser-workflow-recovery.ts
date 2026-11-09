import type { ControlClient, ResourceVersion } from '@/shared/api/control-client.ts';
import { ControlCallError } from '@/shared/api/control-client.ts';
import type { RemoteBrowserHello, RemoteBrowserTransport } from '@/shared/api/remote-browser-transport.ts';

const MAX_BYTES = 4 * 1024 * 1024;
const MAX_PAGES = 4096;
const MAX_SEQUENCE = 9223372036854775807n;
const encoder = new TextEncoder();

export interface BrowserWorkflowRow {
  graph: Record<string, unknown>;
  version: ResourceVersion;
}
export interface BrowserWorkflowEvent {
  topic: 'workflow.changed';
  subscriptionId: string;
  hostInstanceId: string;
  sequence: string;
  eventId: string;
  resource: ResourceVersion;
  actor: { principalId: string };
  data: Record<string, unknown>;
}
function object(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function id(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= 128 &&
    Array.from(value).every((character) => { const code = character.codePointAt(0)!; return code >= 32 && code !== 127; });
}
function sequence(value: unknown, positive = false): bigint {
  if (typeof value !== 'string' || !/^(0|[1-9][0-9]{0,18})$/.test(value)) throw new Error('invalid remote sequence');
  const number = BigInt(value);
  if (number > MAX_SEQUENCE || (positive && number === 0n)) throw new Error('invalid remote sequence');
  return number;
}
function allowed(hello: RemoteBrowserHello, key: string): boolean {
  const restrictions = hello.resourceRestrictions;
  return !Object.prototype.hasOwnProperty.call(restrictions, 'workflow') || restrictions['workflow']!.includes(key);
}
function version(value: unknown, hello: RemoteBrowserHello): ResourceVersion {
  if (!object(value) || value['kind'] !== 'workflow' || value['scope'] !== 'host' || !id(value['id']) || !allowed(hello, value['id'])) {
    throw new Error('invalid remote workflow version');
  }
  sequence(value['revision'], true);
  return { kind: 'workflow', scope: 'host', id: value['id'], revision: value['revision'] as string };
}
function notification(value: unknown, hello: RemoteBrowserHello): BrowserWorkflowEvent {
  if (!object(value) || value['topic'] !== 'workflow.changed' || !id(value['subscriptionId']) ||
      value['hostInstanceId'] !== hello.hostInstanceId || !id(value['eventId']) ||
      !object(value['actor']) || !id(value['actor']['principalId']) || !object(value['data'])) {
    throw new Error('invalid remote workflow event');
  }
  sequence(value['sequence'], true);
  const resource = version(value['resource'], hello);
  if (value['data']['id'] !== resource.id) throw new Error('invalid remote workflow event');
  return {
    topic: 'workflow.changed', subscriptionId: value['subscriptionId'], hostInstanceId: hello.hostInstanceId,
    sequence: value['sequence'] as string, eventId: value['eventId'], resource,
    actor: { principalId: value['actor']['principalId'] }, data: structuredClone(value['data']),
  };
}

/**
 * Workflow-only browser recovery primitive. A complete, identity-bound snapshot
 * must be obtained before data is exposed. Events retain their resource and
 * sequence and must be applied by the caller before acknowledgement. This is
 * not a command dispatcher or a UI adapter.
 */
export function createBrowserWorkflowRecovery(
  client: ControlClient,
  transport: RemoteBrowserTransport,
  hello: RemoteBrowserHello,
  onChange: () => void = () => {},
) {
  let stopped = false;
  let valid = true;
  let started = false;
  let ready = false;
  let subscription: string | undefined;
  let applied = 0n;
  let lastReceived = 0n;
  let rows: BrowserWorkflowRow[] | undefined;
  const aborter = new AbortController();
  const early: BrowserWorkflowEvent[] = [];
  const queue: BrowserWorkflowEvent[] = [];
  let queuedBytes = 0;
  const signal = (): void => { try { onChange(); } catch { /* view callbacks cannot affect recovery state */ } };
  const invalidate = (): void => {
    if (!valid) return;
    valid = false; ready = false; rows = undefined; subscription = undefined;
    early.length = 0; queue.length = 0; queuedBytes = 0; aborter.abort(); signal();
  };
  const current = (): void => {
    if (stopped || !valid || transport.currentHello() !== hello) { invalidate(); throw new Error('browser recovery invalidated'); }
  };
  const accept = (event: BrowserWorkflowEvent): boolean => {
    if (event.subscriptionId !== subscription) throw new Error('remote subscription changed');
    const number = sequence(event.sequence, true);
    if (number <= applied) return false;
    if (number !== lastReceived + 1n) throw new Error('remote event sequence gap');
    lastReceived = number;
    queue.push(event);
    return true;
  };
  const unlistenEvent = transport.onReliableEvent((raw) => {
    if (!valid || stopped) return;
    try {
      const event = notification(raw, hello);
      const size = encoder.encode(JSON.stringify(event)).length;
      if (early.length + queue.length >= MAX_PAGES || queuedBytes + size > MAX_BYTES) throw new Error('remote event buffer overflow');
      if (!ready) { queuedBytes += size; early.push(event); }
      else if (accept(event)) { queuedBytes += size; signal(); }
    } catch { invalidate(); }
  });
  const unlistenLoss = transport.onConnectionLost(() => invalidate());
  async function read(method: string, params: Record<string, unknown>, deadline: number): Promise<unknown> {
    for (let attempt = 0; ; attempt++) {
      current();
      const remaining = Math.floor(deadline - performance.now());
      if (remaining <= 0) throw new Error('remote recovery timed out');
      try {
        const result = await client.call(method, params, { timeoutMs: Math.min(remaining, 30_000), signal: aborter.signal });
        current(); return result;
      } catch (error) {
        current();
        if (!(error instanceof ControlCallError) || error.code !== 'rate_limited' || attempt >= 4) throw error;
        const data = object(error.data) ? error.data : {};
        const proposed = typeof data['retryAfterMs'] === 'number' ? data['retryAfterMs'] : 1000;
        const delay = Math.max(1000, Math.min(2000, Number.isFinite(proposed) ? proposed : 1000));
        if (performance.now() + delay >= deadline) throw new Error('remote recovery timed out', { cause: error });
        await new Promise<void>((resolve, reject) => {
          const timer = setTimeout(() => { aborter.signal.removeEventListener('abort', cancel); resolve(); }, delay);
          const cancel = (): void => { clearTimeout(timer); aborter.signal.removeEventListener('abort', cancel); reject(new Error('browser recovery invalidated')); };
          aborter.signal.addEventListener('abort', cancel, { once: true });
          if (aborter.signal.aborted) cancel();
        });
      }
    }
  }
  return {
    async start(): Promise<void> {
      if (started) throw new Error('browser recovery already started');
      started = true;
      try {
        current();
        if (!hello.capabilities.includes('workflows.read') || !hello.subscriptionDomains.includes('workflows') || hello.features['subscriptions'] !== true) {
          throw new Error('remote workflow recovery unavailable');
        }
        // The host retains a snapshot for 30 seconds, starting after subscribe.
        const sub = await read('collaboration.subscribe', { domains: ['workflows'], feed: false }, performance.now() + 30_000);
        if (!object(sub) || !id(sub['subscriptionId']) || sub['hostInstanceId'] !== hello.hostInstanceId ||
            sub['grantRevision'] !== hello.grantRevision || !Array.isArray(sub['domains']) ||
            sub['domains'].length !== 1 || sub['domains'][0] !== 'workflows' || sub['pointsLimit'] != null) throw new Error('invalid remote subscription');
        subscription = sub['subscriptionId'];
        const deadline = performance.now() + 30_000;
        let token: string | undefined;
        let snapshotId: string | undefined;
        let through: string | undefined;
        let total = 0;
        const tokens = new Set<string>();
        const seen = new Set<string>();
        const baseline: BrowserWorkflowRow[] = [];
        do {
          const page = await read('collaboration.snapshot', { subscriptionId: subscription, ...(token ? { pageToken: token } : {}) }, deadline);
          if (!object(page) || page['subscriptionId'] !== subscription || page['hostInstanceId'] !== hello.hostInstanceId ||
              page['grantRevision'] !== hello.grantRevision || !id(page['snapshotId']) ||
              !Array.isArray(page['domains']) || page['domains'].length !== 1 || page['domains'][0] !== 'workflows' ||
              page['pointsLimit'] != null || !Array.isArray(page['workflows']) || !Array.isArray(page['resourceVersions']) ||
              !Object.prototype.hasOwnProperty.call(page, 'nextPageToken') ||
              (page['points'] !== undefined && (!Array.isArray(page['points']) || page['points'].length !== 0)) ||
              (page['automation'] !== undefined && (!Array.isArray(page['automation']) || page['automation'].length !== 0))) throw new Error('invalid remote snapshot');
          sequence(page['throughSequence']);
          if (snapshotId === undefined) { snapshotId = page['snapshotId']; through = page['throughSequence'] as string; }
          if (page['snapshotId'] !== snapshotId || page['throughSequence'] !== through) throw new Error('remote snapshot changed');
          total += encoder.encode(JSON.stringify(page)).length;
          if (total > MAX_BYTES) throw new Error('remote snapshot too large');
          const bodies = new Map<string, Record<string, unknown>>();
          for (const body of page['workflows']) {
            if (!object(body) || !id(body['id']) || !allowed(hello, body['id']) || bodies.has(body['id']) || seen.has(body['id'])) throw new Error('invalid remote workflow');
            bodies.set(body['id'], body);
          }
          for (const raw of page['resourceVersions']) {
            const item = version(raw, hello);
            const body = bodies.get(item.id);
            if (!body || seen.has(item.id)) throw new Error('invalid remote workflow version');
            bodies.delete(item.id); seen.add(item.id);
            baseline.push({ graph: structuredClone(body), version: item });
          }
          if (bodies.size) throw new Error('missing remote workflow version');
          const next = page['nextPageToken'];
          if (next == null) token = undefined;
          else {
            if (!id(next) || tokens.has(next) || tokens.size >= MAX_PAGES) throw new Error('invalid snapshot pagination');
            tokens.add(next); token = next;
          }
        } while (token !== undefined);
        current();
        applied = sequence(through);
        lastReceived = applied;
        for (const event of early.splice(0)) {
          if (!accept(event)) queuedBytes -= encoder.encode(JSON.stringify(event)).length;
        }
        rows = baseline; ready = true; signal();
      } catch (error) { invalidate(); throw error; }
    },
    baseline(): BrowserWorkflowRow[] | null {
      if (transport.currentHello() !== hello) invalidate();
      return ready && valid && rows ? structuredClone(rows) : null;
    },
    next(): BrowserWorkflowEvent | null {
      if (transport.currentHello() !== hello) invalidate();
      return ready && valid && queue.length ? structuredClone(queue[0]!) : null;
    },
    acknowledge(event: BrowserWorkflowEvent): void {
      current();
      const pending = queue[0];
      if (!ready || !pending || pending.eventId !== event.eventId || pending.sequence !== event.sequence) throw new Error('invalid event acknowledgement');
      applied = sequence(pending.sequence, true);
      queuedBytes -= encoder.encode(JSON.stringify(pending)).length;
      queue.shift(); signal();
    },
    close(): void { if (stopped) return; invalidate(); stopped = true; unlistenEvent(); unlistenLoss(); },
  };
}
