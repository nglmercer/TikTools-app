import type { ControlTransport } from './control-transport.ts';
import type { HostMessage } from '../../../shared/messages.ts';

declare global {
  interface Window {
    ipc?: { postMessage: (message: string) => void };
    __webview_on_message__?: (message: string) => void;
    __tiktools_receive_batch__?: (messages: string[], generation?: number) => void;
    __tiktools_host_message_queue__?: string[];
    __tiktools_presentation_revision__?: number;
  }
}

export type RpcParams = Record<string, unknown>;

export class ControlCallError extends Error {
  readonly code: string;
  readonly data?: unknown;

  constructor(code: string, message: string, data?: unknown) {
    super(message);
    this.name = 'ControlCallError';
    this.code = code;
    this.data = data;
  }
}

export function errorMessage(failure: unknown): string {
  return failure instanceof Error ? failure.message : String(failure);
}

export type TopicHandler<T = unknown> = (data: T) => void;
export type PushHandler = (message: HostMessage) => void;
export type TransportErrorHandler = (message: string) => void;
export type GapHandler = (lost: number) => void;
export type Unsubscribe = () => void;

type PendingCall = {
  resolve: (result: unknown) => void;
  reject: (error: ControlCallError) => void;
  timer: ReturnType<typeof setTimeout>;
  cleanupAbort: () => void;
};

type RpcResponseWire = {
  id?: number | string | null;
  result?: unknown;
  error?: { code?: unknown; message?: unknown; data?: unknown };
};

const DEFAULT_TIMEOUT_MS = 150_000;
const MAX_PENDING_CALLS = 128;
// Mirrors MAX_REQUEST_BYTES in crates/tiktools-control-api/src/transport.rs

const MAX_WEBVIEW_REQUEST_BYTES = 1024 * 1024;

const textEncoder = new TextEncoder();

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

export interface PresentationView {
  feed: boolean;
  behavior: boolean;
  points: boolean;
  visible: boolean;
}

export interface ControlClient {
  setPresentation?(view: PresentationView): Promise<void>;
  diagnostics?(): { pending: number; capacity: number; overloaded: number; topicCount?: number; topicSubscriptions?: number; pushSubscriptions?: number };
  attach(): void;
  detach(): void;
  call<T = unknown>(method: string, params?: RpcParams, options?: { signal?: AbortSignal }): Promise<T>;
  onTopic<T = unknown>(topic: string, handler: TopicHandler<T>): Unsubscribe;
  onPush(type: HostMessage['type'], handler: PushHandler): Unsubscribe;
  onTransportError(handler: TransportErrorHandler): Unsubscribe;
  onGap(handler: GapHandler): Unsubscribe;
}

export function createControlClient(options?: { timeoutMs?: number; transport?: ControlTransport }): ControlClient {
  const transport = options?.transport;
  const timeoutMs = options?.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  let nextId = 0;
  const requestPrefix = globalThis.crypto?.randomUUID?.() ??
    `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  let presentationRevision = 0;
  let presentationGeneration: number | null = null;
  let projectionBaselineSet = false;
  let parsedMessages = 0;
  let parsedBytes = 0;
  let handledMessages = 0;
  let ignoredMessages = 0;
  let batches = 0;
  let parseMs = 0;
  let maxBatchMs = 0;
  const longTasksSupported = typeof PerformanceObserver !== 'undefined' && (PerformanceObserver.supportedEntryTypes?.includes('longtask') ?? false);
  let longTasks = 0;
  let longTaskMs = 0;
  let longTaskObserver: PerformanceObserver | null = null;
  let overloaded = 0;
  let detached = false;
  const pending = new Map<number | string, PendingCall>();
  const topics = new Map<string, Set<TopicHandler<unknown>>>();
  const pushes = new Map<string, Set<PushHandler>>();
  const transportErrors = new Set<TransportErrorHandler>();
  const gaps = new Set<GapHandler>();
  let attachedReceive: ((raw: string) => void) | undefined;

  const reportTransportError = (message: string): void => {
    for (const handler of transportErrors) {
      try {
        handler(message);
      } catch {}
    }
  };

  const receive = (raw: string): void => {
    let value: unknown;
    const parseStarted = performance.now();
    parsedMessages += 1;
    parsedBytes += raw.length;
    try {
      value = JSON.parse(raw) as unknown;
    } catch {
      return;
    }
    parseMs += performance.now() - parseStarted;
    if (!isRecord(value)) return;
    if (value['type'] === 'rpc-response' && isRecord(value['response'])) {
      const response = value['response'] as RpcResponseWire;
      if (response.id === undefined || response.id === null) return;
      const call = pending.get(response.id);
      if (!call) return;
      handledMessages += 1;
      pending.delete(response.id);
      clearTimeout(call.timer);
      call.cleanupAbort();
      if (response.error !== undefined && response.error !== null) {
        const code = typeof response.error.code === 'string' ? response.error.code : 'error';
        const message =
          typeof response.error.message === 'string' ? response.error.message : 'request failed';
        call.reject(new ControlCallError(code, message, response.error.data));
      } else {
        call.resolve(response.result ?? null);
      }
      return;
    }
    if (value['method'] === 'event.gap' && isRecord(value['params'])) {
      const params = value['params'] as { lost?: unknown };
      const lost = typeof params.lost === 'number' ? params.lost : 0;
      for (const handler of gaps) {
        try {
          handler(lost);
        } catch {}
      }
      return;
    }
    if (value['method'] === 'event' && isRecord(value['params'])) {
      const params = value['params'] as { topic?: unknown; data?: unknown; revision?: unknown; generation?: unknown };
      if (typeof params.generation === 'number' && params.generation !== presentationGeneration) { ignoredMessages += 1; return; }
      if (typeof params.revision === 'number' && params.revision !== presentationRevision) { ignoredMessages += 1; return; }
      if (typeof params.topic !== 'string') return;
      const handlers = topics.get(params.topic);
      if (!handlers) { ignoredMessages += 1; return; }
      handledMessages += 1;
      for (const handler of handlers) {
        try {
          handler(params.data);
        } catch {}
      }
      return;
    }
    if (typeof value['type'] === 'string') {
      const handlers = pushes.get(value['type']);
      if (!handlers) { ignoredMessages += 1; return; }
      handledMessages += 1;
      const message = value as unknown as HostMessage;
      for (const handler of handlers) {
        try {
          handler(message);
        } catch {}
      }
    }
  };

  const receiveBatch = (messages: string[], generation?: number): void => {
    if (generation !== undefined && presentationGeneration !== null && generation !== presentationGeneration) {
      ignoredMessages += messages.length;
      return;
    }
    batches += 1;
    const started = performance.now();
    for (const item of messages) receive(item);
    maxBatchMs = Math.max(maxBatchMs, performance.now() - started);
  };

  const client: ControlClient = {
    async setPresentation(view): Promise<void> {
      const nextRevision = (): number => {
        const next = Math.max(presentationRevision, window.__tiktools_presentation_revision__ ?? 0) + 1;
        presentationRevision = next;
        window.__tiktools_presentation_revision__ = next;
        return next;
      };
      let revision = nextRevision();
      const configure = (revision: number) => client.call<{ stats?: unknown; generation?: number }>('ui.projection', { version: 1, revision, ...view });
      let response;
      try { response = await configure(revision); }
      catch (failure) {
        const hostRevision = failure instanceof ControlCallError && isRecord(failure.data) ? failure.data['revision'] : undefined;
        if (!(failure instanceof ControlCallError) || failure.code !== 'stale' ||
            presentationGeneration !== null || revision !== presentationRevision ||
            typeof hostRevision !== 'number' || !Number.isSafeInteger(hostRevision) || hostRevision < 0) throw failure;
        window.__tiktools_presentation_revision__ = Math.max(window.__tiktools_presentation_revision__ ?? 0, hostRevision);
        revision = nextRevision();
        response = await configure(revision);
      }
      if (revision === presentationRevision && typeof response.generation === 'number') presentationGeneration = response.generation;
      if (!projectionBaselineSet && response.stats && revision === presentationRevision) {
        projectionBaselineSet = true;
        for (const handler of topics.get('ui.projection.stats') ?? []) handler(response.stats);
      }
    },
    diagnostics: () => ({ parsedMessages, parsedStringUnits: parsedBytes, handledMessages, ignoredMessages, batches, parseMs, maxBatchMs, longTasksSupported, longTasks, longTaskMs, pending: pending.size, capacity: MAX_PENDING_CALLS, overloaded,
      topicCount: topics.size, topicSubscriptions: [...topics.values()].reduce((total, handlers) => total + handlers.size, 0),
      pushSubscriptions: [...pushes.values()].reduce((total, handlers) => total + handlers.size, 0),
    }),
    attach(): void {
      detached = false;
      attachedReceive = receive;
      if (longTasksSupported) {
        longTaskObserver?.disconnect();
        longTaskObserver = new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) { longTasks += 1; longTaskMs += entry.duration; }
        });
        longTaskObserver.observe({ entryTypes: ['longtask'] });
      }
      if (transport) {
        transport.attach(receive, (reason) => {
          for (const call of pending.values()) {
            clearTimeout(call.timer);
            call.cleanupAbort();
            call.reject(new ControlCallError('transport', reason));
          }
          pending.clear();
          reportTransportError(reason);
        }, () => {
          for (const handler of gaps) { try { handler(0); } catch {} }
        });
        return;
      }
      window.__webview_on_message__ = receive;
      window.__tiktools_receive_batch__ = receiveBatch;
      const queued = window.__tiktools_host_message_queue__ ?? [];
      window.__tiktools_host_message_queue__ = [];
      queued.forEach(receive);
    },

    detach(): void {
      detached = true;
      transport?.detach();
      longTaskObserver?.disconnect();
      longTaskObserver = null;
      if (window.__webview_on_message__ === attachedReceive) {
        window.__webview_on_message__ = undefined;
      }
      if (window.__tiktools_receive_batch__ === receiveBatch) {
        window.__tiktools_receive_batch__ = undefined;
      }
      attachedReceive = undefined;
      topics.clear();
      pushes.clear();
      transportErrors.clear();
      gaps.clear();
      for (const [, call] of pending) {
        clearTimeout(call.timer);
        call.cleanupAbort();
        call.reject(new ControlCallError('transport', 'control client detached'));
      }
      pending.clear();
    },

    call<T = unknown>(method: string, params?: RpcParams, options?: { signal?: AbortSignal }): Promise<T> {
      return new Promise<T>((resolve, reject) => {
        const signal = options?.signal;
        if (signal?.aborted) { reject(new ControlCallError('cancelled', 'control request owner disposed')); return; }
        if (detached) {
          reject(new ControlCallError('transport', 'control client detached'));
          return;
        }
        if (pending.size >= MAX_PENDING_CALLS) {
          overloaded += 1;
          const error = new ControlCallError('overloaded', 'control client pending request limit reached');
          reportTransportError(error.message);
          reject(error);
          return;
        }
        if (!transport && !window.ipc) {
          reject(new ControlCallError('transport', 'native host is unavailable'));
          return;
        }
        const id = `rpc-${requestPrefix}-${++nextId}`;
        let payload: string;
        try {
          payload = JSON.stringify({ jsonrpc: '2.0', id, method, params: params ?? {},
            ...(presentationGeneration === null ? {} : { pageGeneration: presentationGeneration }),
          });
        } catch (error) {
          reject(
            new ControlCallError('transport', `could not serialize ${method}: ${String(error)}`),
          );
          return;
        }
        if (textEncoder.encode(payload).length > MAX_WEBVIEW_REQUEST_BYTES) {
          const error = new ControlCallError(
            'too_large',
            `request ${method} exceeds the ${MAX_WEBVIEW_REQUEST_BYTES}-byte limit`,
          );
          reportTransportError(error.message);
          reject(error);
          return;
        }
        const onAbort = (): void => {
          if (!pending.delete(id)) return;
          clearTimeout(timer);
          cleanupAbort();
          reject(new ControlCallError('cancelled', 'control request owner disposed; accepted operation may still complete'));
        };
        const cleanupAbort = (): void => { signal?.removeEventListener('abort', onAbort); };
        const timer = setTimeout(() => {
          pending.delete(id);
          cleanupAbort();
          const error = new ControlCallError('timeout', `request ${method} timed out`);
          reportTransportError(error.message);
          reject(error);
        }, timeoutMs);
        pending.set(id, {
          resolve: (result) => resolve(result as T),
          reject,
          timer,
          cleanupAbort,
        });
        signal?.addEventListener('abort', onAbort, { once: true });
        if (signal?.aborted) { onAbort(); return; }
        try {
          if (transport) transport.send(payload);
          else window.ipc!.postMessage(payload);
        } catch (error) {
          pending.delete(id);
          clearTimeout(timer);
          cleanupAbort();
          reject(new ControlCallError('transport', `could not send ${method}: ${String(error)}`));
        }
      });
    },

    onTopic<T = unknown>(topic: string, handler: TopicHandler<T>): Unsubscribe {
      let handlers = topics.get(topic);
      if (!handlers) {
        handlers = new Set();
        topics.set(topic, handlers);
      }
      const erased = handler as TopicHandler<unknown>;
      handlers.add(erased);
      return () => {
        handlers.delete(erased);
        if (handlers.size === 0 && topics.get(topic) === handlers) topics.delete(topic);
      };
    },

    onPush(type, handler): Unsubscribe {
      let handlers = pushes.get(type);
      if (!handlers) {
        handlers = new Set();
        pushes.set(type, handlers);
      }
      handlers.add(handler);
      return () => {
        handlers.delete(handler);
        if (handlers.size === 0 && pushes.get(type) === handlers) pushes.delete(type);
      };
    },

    onTransportError(handler): Unsubscribe {
      transportErrors.add(handler);
      return () => {
        transportErrors.delete(handler);
      };
    },

    onGap(handler): Unsubscribe {
      gaps.add(handler);
      return () => {
        gaps.delete(handler);
      };
    },
  };
  // Presentation projection is owned by the desktop window, not the IPC host.
  if (transport) delete client.setPresentation;
  return client;
}

export function notifyFrontendReady(): void {
  if (!window.ipc) {
    throw new Error('TikTools native IPC bridge is unavailable during frontend startup');
  }
  window.ipc.postMessage(JSON.stringify({ type: 'frontend-ready' }));
}
