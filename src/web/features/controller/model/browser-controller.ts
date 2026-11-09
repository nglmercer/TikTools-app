import { ControlCallError, type ControlClient, type ResourceVersion, type RpcParams } from '@/shared/api/control-client.ts';
import type { RemoteBrowserHello, RemoteBrowserTransport } from '@/shared/api/remote-browser-transport.ts';
import { createBrowserWorkflowRecovery, type BrowserWorkflowRow } from './browser-workflow-recovery.ts';
import type { ControllerStatus } from './controller.ts';

const MAX_VIEW_BYTES = 4 * 1024 * 1024;
const PAGE_BYTES = 1024 * 1024;
const encoder = new TextEncoder();
function fail(code: string, message: string): ControlCallError { return new ControlCallError(code, message); }
function object(value: unknown): value is Record<string, unknown> { return typeof value === 'object' && value !== null && !Array.isArray(value); }
function revision(value: unknown): bigint {
  if (typeof value !== 'string' || !/^(0|[1-9][0-9]{0,18})$/.test(value)) throw fail('protocol', 'Invalid workflow revision');
  const number = BigInt(value);
  if (number > 9223372036854775807n) throw fail('protocol', 'Invalid workflow revision');
  return number;
}

/** Browser-local bridge for the existing controller UI. Only workflow reads
 * are exposed until browser command journaling and the other domains exist. */
export function createBrowserControllerClient(remote: ControlClient, transport: RemoteBrowserTransport, origin: string): ControlClient {
  const url = new URL(origin);
  if (url.origin !== origin || url.protocol !== 'https:' || url.username || url.password) throw new Error('Invalid browser host origin');
  let epoch = 0;
  let viewRevision = 0;
  let phase: 'idle' | 'connecting' | 'ready' | 'failed' = 'idle';
  let errorCode: string | null = null;
  let hello: RemoteBrowserHello | undefined;
  let recovery: ReturnType<typeof createBrowserWorkflowRecovery> | undefined;
  let rows: BrowserWorkflowRow[] = [];
  let processing = false;
  let attached = false;
  let unlistenLoss: (() => void) | undefined;
  const listeners = new Set<(data: unknown) => void>();
  const notify = (): void => { for (const listener of listeners) { try { listener({}); } catch { /* one view cannot block invalidation */ } } };
  const status = (): ControllerStatus => ({
    epoch: String(epoch), viewRevision: String(viewRevision), phase, profile: phase === 'idle' ? null : 'browser',
    origin: phase === 'idle' ? null : origin, errorCode, draftsSupported: false, proposalsSupported: false,
    commandJournal: null, commandBusy: false,
    hello: hello && phase === 'ready' ? {
      hostId: hello.hostId, hostInstanceId: hello.hostInstanceId, connectionId: hello.connectionId,
      principalId: hello.principalId, sessionId: hello.sessionId, grantRevision: hello.grantRevision,
      // Advertise only domains that this browser bridge actually implements.
      capabilities: hello.capabilities.includes('workflows.read') ? ['workflows.read'] : [],
      features: { subscriptions: true, workflowDisable: false, workflowDelete: false, workflowSave: false, workflowEnable: false },
      resourceRestrictions: Object.prototype.hasOwnProperty.call(hello.resourceRestrictions, 'workflow')
        ? { workflow: [...hello.resourceRestrictions['workflow']!] } : {},
      limits: { requestsPerSecond: hello.requestsPerSecond, requestBurst: hello.requestBurst },
    } : null,
  });
  function invalidate(code: string, state: 'idle' | 'failed'): void {
    epoch += 1; viewRevision += 1; phase = state; errorCode = state === 'failed' ? code : null;
    const previous = recovery; recovery = undefined; rows = []; hello = undefined;
    previous?.close(); notify();
  }
  function matches(generation: number, expected: RemoteBrowserHello): boolean {
    return attached && epoch === generation && phase === 'ready' && hello === expected && transport.currentHello() === expected;
  }
  async function pump(): Promise<void> {
    if (processing || phase !== 'ready' || !recovery || !hello) return;
    processing = true;
    const generation = epoch;
    const expected = hello;
    const stream = recovery;
    try {
      while (matches(generation, expected) && recovery === stream) {
        const event = stream.next();
        if (!event) {
          if (!stream.isReady()) throw fail('resync_required', 'Workflow recovery is no longer available');
          break;
        }
        let graph: unknown;
        let versions: unknown;
        try {
          graph = await remote.call('workflows.get', { id: event.resource.id }, {
            timeoutMs: 15_000, onMetadata: (meta) => { versions = meta?.resourceVersions; },
          });
        } catch (error) {
          if (!(error instanceof ControlCallError) || error.code !== 'workflow_not_found' || !object(error.data)) throw error;
          graph = null; versions = error.data['resourceVersions'];
        }
        if (!matches(generation, expected) || recovery !== stream) return;
        if (!Array.isArray(versions)) throw fail('protocol', 'Missing workflow version');
        const found = versions.filter((item): item is ResourceVersion => object(item) && item['kind'] === 'workflow' && item['scope'] === 'host' && item['id'] === event.resource.id && typeof item['revision'] === 'string');
        if (found.length !== 1 || revision(found[0]!.revision) < revision(event.resource.revision) ||
            (graph !== null && (!object(graph) || graph['id'] !== event.resource.id))) throw fail('protocol', 'Invalid workflow update');
        const previous = rows.find((row) => row.version.id === event.resource.id);
        if (!previous || revision(found[0]!.revision) >= revision(previous.version.revision)) {
          const next = rows.filter((row) => row.version.id !== event.resource.id);
          if (graph !== null) next.push({ graph: structuredClone(graph as Record<string, unknown>), version: structuredClone(found[0]!) });
          next.sort((a, b) => a.version.id < b.version.id ? -1 : a.version.id > b.version.id ? 1 : 0);
          if (encoder.encode(JSON.stringify(next)).length > MAX_VIEW_BYTES) throw fail('too_large', 'Workflow view exceeds its limit');
          rows = next; viewRevision += 1;
        }
        stream.acknowledge(event); notify();
      }
    } catch {
      if (matches(generation, expected)) { invalidate('resync_required', 'failed'); transport.disconnect(); }
    } finally {
      processing = false;
      // A stale request may finish after a new connection became ready.
      // Resume its queued updates rather than leaving them stranded.
      if (phase === 'ready' && recovery?.next()) void pump();
    }
  }
  async function connect(): Promise<void> {
    if (!attached || phase === 'connecting' || phase === 'ready') throw fail('invalid_state', 'Browser controller is already connected');
    const generation = ++epoch;
    viewRevision += 1; phase = 'connecting'; errorCode = null; rows = []; hello = undefined; notify();
    try {
      const negotiated = await transport.connect();
      if (!attached || generation !== epoch) throw fail('cancelled', 'Browser connection was replaced');
      const stream = createBrowserWorkflowRecovery(remote, transport, negotiated, () => {
        if (generation !== epoch || recovery !== stream) return;
        if (phase === 'ready' && !stream.isReady()) { invalidate('resync_required', 'failed'); transport.disconnect(); }
        else void pump();
      });
      recovery = stream;
      await stream.start();
      if (!attached || generation !== epoch || transport.currentHello() !== negotiated || recovery !== stream) throw fail('cancelled', 'Browser connection was replaced');
      const baseline = stream.baseline();
      if (!baseline) throw fail('resync_required', 'Workflow baseline unavailable');
      rows = baseline.sort((a, b) => a.version.id < b.version.id ? -1 : a.version.id > b.version.id ? 1 : 0);
      hello = negotiated; phase = 'ready'; viewRevision += 1; notify(); void pump();
    } catch (error) {
      if (generation === epoch) { invalidate(error instanceof ControlCallError ? error.code : 'connection_failed', 'failed'); transport.disconnect(); }
      throw error;
    }
  }
  const bridge: ControlClient = {
    ...remote,
    attach() {
      if (attached) return;
      attached = true;
      remote.attach();
      unlistenLoss = transport.onConnectionLost(() => { if (phase === 'ready' || phase === 'connecting') invalidate('transport', 'failed'); });
    },
    detach() {
      if (!attached) return;
      attached = false;
      unlistenLoss?.(); unlistenLoss = undefined;
      invalidate('transport', 'idle');
      remote.detach(); listeners.clear();
    },
    async call<T = unknown>(method: string, params: RpcParams = {}): Promise<T> {
      if (!attached) throw fail('transport', 'Browser controller is unavailable');
      if (method === 'controller.status') return status() as T;
      if (method === 'controller.profiles') return [] as T;
      if (method === 'controller.connect') {
        if (params['profile'] !== 'browser') throw fail('invalid_params', 'Invalid browser profile');
        await connect(); return { ok: true } as T;
      }
      if (method === 'controller.disconnect') { invalidate('transport', 'idle'); transport.disconnect(); return { ok: true } as T; }
      if (method === 'controller.workflows') {
        if (phase !== 'ready' || params['epoch'] !== String(epoch) || params['viewRevision'] !== String(viewRevision)) throw fail('stale_view', 'Workflow view changed');
        const offset = params['offset'];
        if (typeof offset !== 'number' || !Number.isSafeInteger(offset) || offset < 0 || offset > rows.length) throw fail('invalid_params', 'Invalid workflow page');
        const page: BrowserWorkflowRow[] = [];
        let used = 0;
        for (let i = offset; i < rows.length && page.length < 100; i++) {
          const item = rows[i]!; const bytes = encoder.encode(JSON.stringify(item)).length;
          if (page.length && used + bytes > PAGE_BYTES) break;
          if (bytes > PAGE_BYTES) throw fail('too_large', 'Workflow exceeds the page limit');
          page.push(item); used += bytes;
        }
        const next = offset + page.length < rows.length ? offset + page.length : null;
        return { epoch: String(epoch), viewRevision: String(viewRevision), rows: structuredClone(page), next } as T;
      }
      throw fail('unsupported_remote', 'This browser operation is not available');
    },
    onTopic<T = unknown>(topic: string, handler: (data: T) => void) {
      if (topic !== 'controller.changed') return () => {};
      const listener = handler as (data: unknown) => void;
      listeners.add(listener); return () => { listeners.delete(listener); };
    },
  };
  return bridge;
}
