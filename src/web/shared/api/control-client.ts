import type { ControlTransport } from './control-transport.ts';
import type { HostMessage } from '../../../shared/messages.ts';
import { ControlCallError, type ControlClient, type TopicHandler, type PushHandler, type TransportErrorHandler, type GapHandler, type Unsubscribe } from './control-contracts.ts';
import { isRecord, type RpcResponseWire } from './control-wire.ts';
import { createControlRequests } from './control-requests.ts';
export * from './control-contracts.ts';

declare global {
  interface Window {
    ipc?: { postMessage: (message: string) => void };
    __webview_on_message__?: (message: string) => void;
    __tiktools_receive_batch__?: (messages: string[], generation?: number) => void;
    __tiktools_host_message_queue__?: string[];
    __tiktools_presentation_revision__?: number;
  }
}

export function createControlClient(options?: { timeoutMs?: number; transport?: ControlTransport }): ControlClient {
  const transport = options?.transport;
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
  let detached = false;
  const topics = new Map<string, Set<TopicHandler<unknown>>>();
  const pushes = new Map<string, Set<PushHandler>>();
  const transportErrors = new Set<TransportErrorHandler>();
  const gaps = new Set<GapHandler>();
  let attachedReceive: ((raw: string) => void) | undefined;

  /** Isolate subscriber failures from the transport loop, but never hide them. */
  const notify = <T>(label: string, handler: (value: T) => void, value: T): void => {
    try {
      handler(value);
    } catch (error) {
      console.error(`control-client ${label} handler failed`, error);
    }
  };

  const reportTransportError = (message: string): void => {
    for (const handler of transportErrors) notify('transport-error', handler, message);
  };

  const requests = createControlRequests({
    transport,
    timeoutMs: options?.timeoutMs,
    generation: () => presentationGeneration,
    detached: () => detached,
    reportTransportError,
    handledResponse: () => { handledMessages += 1; },
  });

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
      requests.settle(value['response'] as RpcResponseWire);
      return;
    }
    if (value['method'] === 'event.gap' && isRecord(value['params'])) {
      const params = value['params'] as { lost?: unknown };
      const lost = typeof params.lost === 'number' ? params.lost : 0;
      for (const handler of gaps) notify('event.gap', handler, lost);
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
      for (const handler of handlers) notify(`topic ${params.topic}`, handler, params.data);
      return;
    }
    if (typeof value['type'] === 'string') {
      const handlers = pushes.get(value['type']);
      if (!handlers) { ignoredMessages += 1; return; }
      handledMessages += 1;
      const message = value as unknown as HostMessage;
      for (const handler of handlers) notify(`push ${message.type}`, handler, message);
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
    diagnostics: () => ({ parsedMessages, parsedStringUnits: parsedBytes, handledMessages, ignoredMessages, batches, parseMs, maxBatchMs, longTasksSupported, longTasks, longTaskMs, ...requests.diagnostics(),
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
          requests.rejectAll(reason);
          reportTransportError(reason);
        }, () => {
          for (const handler of gaps) notify('event.gap', handler, 0);
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
      requests.rejectAll('control client detached');
    },

    call: requests.call,

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
