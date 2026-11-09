import { ControlCallError, type ControlClient, type ResourceVersion, type RpcParams } from '@/shared/api/control-client.ts';
import type { RemoteBrowserHello, RemoteBrowserTransport } from '@/shared/api/remote-browser-transport.ts';
import { createBrowserWorkflowRecovery, type BrowserWorkflowRow } from './browser-workflow-recovery.ts';
import type { ControllerStatus } from './controller.ts';
import { finishBrowserCommand, initializeBrowserCommandJournal, withBrowserCommandJournal, withBrowserCommandLock, type BrowserCommandBinding } from './browser-command-journal.ts';
import { recoverBrowserWorkflowCommand } from './browser-command-recovery.ts';
import { sendBrowserWorkflowCommand } from './browser-workflow-command.ts';

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

/** Browser-local bridge for scoped reads and journaled workflow commands. */
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
  let commandBusy = false;
  let unlistenLoss: (() => void) | undefined;
  const listeners = new Set<(data: unknown) => void>();
  const notify = (): void => { for (const listener of listeners) { try { listener({}); } catch { /* one view cannot block invalidation */ } } };
  const status = (): ControllerStatus => ({
    epoch: String(epoch), viewRevision: String(viewRevision), phase, profile: phase === 'idle' ? null : 'browser',
    origin: phase === 'idle' ? null : origin, errorCode, draftsSupported: false, proposalsSupported: false,
    commandJournal: null, commandBusy,
    hello: hello && phase === 'ready' ? {
      hostId: hello.hostId, hostInstanceId: hello.hostInstanceId, connectionId: hello.connectionId,
      principalId: hello.principalId, sessionId: hello.sessionId, grantRevision: hello.grantRevision,
      // Advertise only domains that this browser bridge actually implements.
      capabilities: hello.capabilities.filter((capability) => ['workflows.read', 'workflows.toggle', 'workflows.edit', 'commands.read', 'live.read'].includes(capability)),
      features: { subscriptions: true, workflowDisable: hello.features['workflowDisable'] === true && hello.capabilities.includes('commands.read'), workflowDelete: hello.features['workflowDelete'] === true && hello.capabilities.includes('commands.read'), workflowSave: false, workflowEnable: false,
        liveFeedSupported: hello.features['browserFeed'] === true,
        creatorFeed: hello.features['creatorFeed'] === true },
      resourceRestrictions: Object.fromEntries(['workflow', 'live'].filter((kind) =>
        Object.prototype.hasOwnProperty.call(hello!.resourceRestrictions, kind))
        .map((kind) => [kind, [...hello!.resourceRestrictions[kind]!]])),
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
  const bindingFor = (expected: RemoteBrowserHello): BrowserCommandBinding => ({ origin, hostId: expected.hostId, principalId: expected.principalId, sessionId: expected.sessionId });
  const hasCommands = (expected: RemoteBrowserHello): boolean => expected.capabilities.includes('commands.read') && expected.capabilities.includes('workflows.read') &&
    ((expected.features['workflowDisable'] === true && expected.capabilities.includes('workflows.toggle')) || (expected.features['workflowDelete'] === true && expected.capabilities.includes('workflows.edit')));
  const permitted = (expected: RemoteBrowserHello, id: string): boolean => expected.capabilities.includes('workflows.read') &&
    (expected.resourceRestrictions['workflow'] === undefined || expected.resourceRestrictions['workflow'].includes(id));
  async function commandStatus(): Promise<ControllerStatus> {
    const expected = hello; const generation = epoch;
    if (!expected || !matches(generation, expected) || !hasCommands(expected)) return status();
    const binding = bindingFor(expected);
    if (!globalThis.navigator?.locks) return { ...status(), commandJournal: { binding: { ...binding, profileId: 'browser' }, pending: null, unavailable: true, recent: [] } };
    try {
      const journal = await withBrowserCommandJournal(binding, (value) => structuredClone(value));
      if (!await transport.verifySession() || !matches(generation, expected)) return status();
      const hidden = !!journal.pending && !permitted(expected, journal.pending.resourceId);
      return { ...status(), commandJournal: { binding: { ...binding, profileId: 'browser' }, pending: hidden ? null : journal.pending,
        unavailable: hidden, recent: journal.recent.filter((item) => permitted(expected, item.intent.resourceId)) } };
    } catch (error) {
      if (!matches(generation, expected)) return status();
      const missing = error instanceof Error && error.message === 'command journal missing; review storage loss';
      return { ...status(), browserJournalMissing: missing, commandJournal: { binding: { ...binding, profileId: 'browser' }, pending: null, unavailable: true, recent: [] } };
    }
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
        if (!await transport.verifySession()) {
          if (matches(generation, expected)) { invalidate('transport', 'failed'); transport.disconnect(); }
          return;
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
      let stream: ReturnType<typeof createBrowserWorkflowRecovery> | undefined;
      if (negotiated.capabilities.includes('workflows.read')) {
        stream = createBrowserWorkflowRecovery(remote, transport, negotiated, () => {
          if (generation !== epoch || recovery !== stream || !stream) return;
          if (phase === 'ready' && !stream.isReady()) { invalidate('resync_required', 'failed'); transport.disconnect(); }
          else void pump();
        });
        recovery = stream;
        await stream.start();
      }
      if (!await transport.verifySession()) throw fail('transport', 'Browser session changed');
      if (!attached || generation !== epoch || transport.currentHello() !== negotiated || recovery !== stream) throw fail('cancelled', 'Browser connection was replaced');
      const baseline = stream ? stream.baseline() : [];
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
      if (method === 'controller.status') return await commandStatus() as T;
      if (method === 'controller.profiles') return [] as T;
      if (method === 'controller.connect') {
        if (params['profile'] !== 'browser') throw fail('invalid_params', 'Invalid browser profile');
        await connect(); return { ok: true } as T;
      }
      if (method === 'controller.disconnect') { invalidate('transport', 'idle'); transport.disconnect(); return { ok: true } as T; }
      if (method === 'controller.command.initialize' || method === 'controller.workflow.command' || method === 'controller.command.resolve' || method === 'controller.command.review') {
        const expected = hello; const generation = epoch;
        if (!expected || !matches(generation, expected) || !hasCommands(expected) || params['epoch'] !== String(epoch) || commandBusy) throw fail('stale_view', 'Command view changed');
        const binding = bindingFor(expected);
        commandBusy = true; notify();
        try {
          if (method === 'controller.command.initialize') {
            if (params['acknowledgeStorageLoss'] !== true || !await transport.verifySession() || !matches(generation, expected)) throw fail('stale_view', 'Storage review changed');
            await withBrowserCommandLock(binding, async () => {
              if (!matches(generation, expected)) throw fail('stale_view', 'Storage review changed');
              await initializeBrowserCommandJournal(binding, true);
            });
          } else if (method === 'controller.workflow.command') {
            const operation = params['operation']; const id = params['id']; const expectedRevision = params['expectedRevision']; const capturedView = params['viewRevision'];
            if ((operation !== 'disable' && operation !== 'delete') || typeof id !== 'string' || typeof expectedRevision !== 'string') throw fail('invalid_params', 'Invalid workflow command');
            const isCurrent = () => matches(generation, expected) && capturedView === String(viewRevision) && permitted(expected, id) && rows.some((row) => row.version.id === id && row.version.revision === expectedRevision && (operation !== 'disable' || row.graph['enabled'] === true));
            if (!isCurrent()) throw fail('stale_view', 'Workflow view changed');
            await sendBrowserWorkflowCommand({ remote, transport, expected, binding, target: { operation, resourceId: id, expectedRevision }, isCurrent });
          } else {
            const commandId = params['commandId'];
            if (typeof commandId !== 'string' || !await transport.verifySession() || !matches(generation, expected)) throw fail('stale_view', 'Pending command changed');
            if (method === 'controller.command.resolve') await recoverBrowserWorkflowCommand(remote, transport, expected, binding, commandId);
            else {
              if (params['acknowledgeUnknown'] !== true) throw fail('invalid_params', 'Review required');
              await withBrowserCommandLock(binding, async () => {
                if (!await transport.verifySession() || !matches(generation, expected)) throw fail('stale_view', 'Pending command changed');
                await withBrowserCommandJournal(binding, (journal) => {
                  if (!matches(generation, expected) || !journal.pending || journal.pending.commandId !== commandId || !permitted(expected, journal.pending.resourceId)) throw fail('stale_view', 'Pending command changed');
                  finishBrowserCommand(journal, commandId, { outcome: 'reviewed-unknown', revision: null, errorCode: null, recordedAt: Date.now() });
                });
              });
            }
          }
          if (!matches(generation, expected)) throw fail('stale_view', 'Command session changed');
          return { ok: true } as T;
        } finally { commandBusy = false; notify(); }
      }
      if (method === 'controller.live.status') {
        const expected = hello; const generation = epoch;
        if (!expected || !matches(generation, expected) || !expected.capabilities.includes('live.read') ||
            params['epoch'] !== String(epoch) || params['connectionId'] !== expected.connectionId) throw fail('stale_view', 'LIVE view changed');
        const value = await remote.call<unknown>('live.status', {});
        const optional = (item: unknown): boolean => item === null || (typeof item === 'string' && item.length > 0 &&
          encoder.encode(item).length <= 256 && !/\p{Cc}/u.test(item));
        if (!object(value) || Object.keys(value).length !== 5 ||
            !['connected', 'uniqueId', 'roomId', 'connectionId', 'native'].every((key) => Object.hasOwn(value, key)) ||
            typeof value['connected'] !== 'boolean' || typeof value['native'] !== 'boolean' ||
            !optional(value['uniqueId']) || !optional(value['roomId']) || !optional(value['connectionId'])) throw fail('protocol', 'Invalid LIVE status');
        const allowed = expected.resourceRestrictions['live'];
        if (allowed !== undefined && (typeof value['uniqueId'] !== 'string' || !allowed.includes(value['uniqueId']))) throw fail('forbidden', 'LIVE creator is not permitted');
        if (!await transport.verifySession() || !matches(generation, expected)) throw fail('stale_view', 'LIVE view changed');
        return { epoch: String(generation), connectionId: expected.connectionId, connected: value['connected'], uniqueId: value['uniqueId'], native: value['native'] } as T;
      }
      if (method === 'controller.live.feed') {
        const expected = hello; const generation = epoch;
        if (!expected || !matches(generation, expected) || !expected.capabilities.includes('live.read') || expected.features['browserFeed'] !== true ||
            params['epoch'] !== String(epoch) || params['connectionId'] !== expected.connectionId) throw fail('stale_view', 'LIVE view changed');
        const after = params['after']; const creator = params['creatorId']; const stream = params['streamId'];
        if (typeof after !== 'string' || !/^(0|[1-9][0-9]{0,19})$/.test(after) || BigInt(after) > 18446744073709551615n) throw fail('invalid_params', 'Invalid LIVE cursor');
        const restrictions = expected.resourceRestrictions['live'];
        let value: unknown;
        if (creator !== undefined) {
          if (expected.features['creatorFeed'] !== true || typeof creator !== 'string' || !creator || creator.trim() !== creator ||
              encoder.encode(creator).length > 256 || /\p{Cc}/u.test(creator) || (restrictions !== undefined && !restrictions.includes(creator)) ||
              !(stream === null || (typeof stream === 'string' && /^[0-9a-f]{32}$/.test(stream))) || (stream === null && after !== '0')) throw fail('forbidden', 'LIVE creator is not permitted');
          value = await remote.call('ui.feed.creator', { creatorId: creator, streamId: stream, after, limit: 64 });
          if (!object(value) || value['creatorId'] !== creator) throw fail('protocol', 'Invalid LIVE feed');
        } else {
          if (restrictions !== undefined || stream !== undefined) throw fail('forbidden', 'LIVE feed is not permitted');
          value = await remote.call('ui.feed.browser', { after });
          if (!object(value)) throw fail('protocol', 'Invalid LIVE feed');
        }
        if (!await transport.verifySession() || !matches(generation, expected)) throw fail('stale_view', 'LIVE view changed');
        return { ...value, epoch: String(generation), connectionId: expected.connectionId } as T;
      }
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
