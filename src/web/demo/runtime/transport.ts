import type { ControlTransport } from '@/shared/api/control-transport.ts';
import type { RequestMetadata, ResponseMetadata } from '@/shared/api/control-contracts.ts';
import { isResponseMetadata } from '@/shared/api/control-wire.ts';

export class DemoRpcError extends Error {
  constructor(readonly code: string, message: string, readonly data?: unknown) { super(message); }
}

export interface DemoReply<T = unknown> { result: T; meta?: ResponseMetadata }
export interface DemoMethod {
  description: string;
  destructive: boolean;
  invoke(params: unknown, meta: RequestMetadata | undefined): DemoReply;
}

/** Each supported operation supplies its own parser and typed result; there is no default success. */
export function demoMethod<P, R>(
  description: string,
  parse: (value: unknown) => P,
  run: (params: P, meta: RequestMetadata | undefined) => DemoReply<R>,
  destructive = false,
): DemoMethod {
  return { description, destructive, invoke: (params, meta) => run(parse(params), meta) };
}

export function objectParams(value: unknown): Record<string, unknown> {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) throw new DemoRpcError('invalid_params', 'Expected an object.');
  return value as Record<string, unknown>;
}

export function stringParam(value: unknown, name: string, max = 512): string {
  if (typeof value !== 'string' || value.length > max) throw new DemoRpcError('invalid_params', `Invalid ${name}.`);
  return value;
}

/** An in-memory JSON-RPC wire peer. It has no network, native IPC, shell or code evaluator. */
export function createDemoTransport(definitions: Readonly<Record<string, DemoMethod>>) {
  const methods = new Map(Object.entries(definitions));
  methods.set('rpc.discover', demoMethod('Discover simulated host capabilities', objectParams, () => ({
    result: { methods: [...methods].map(([name, entry]) => ({ name, description: entry.description, destructive: entry.destructive, sideEffect: entry.destructive, requiresDesktop: false, durableCommands: false, paramsSchema: { type: 'object' }, resultSchema: {} })) },
  })));
  let receive: ((raw: string) => void) | undefined;
  let disconnected: ((reason: string) => void) | undefined;
  let reconnected: (() => void) | undefined;
  let generation = 0;
  let detached = false;
  let lost = false;
  let scheduled = false;
  let queued: string[] = [];
  let queuedBytes = 0;
  let delivered = 0;
  const encoder = new TextEncoder();
  const limit = 128;
  const maxMessageBytes = 2 * 1024 * 1024;
  const maxQueueBytes = 4 * 1024 * 1024;

  function flush(): void {
    if (scheduled || !receive || queued.length === 0) return;
    scheduled = true;
    const mine = generation;
    queueMicrotask(() => {
      if (mine !== generation) return;
      scheduled = false;
      const batch = queued;
      queued = [];
      queuedBytes = 0;
      for (const raw of batch) {
        if (mine !== generation || !receive) return;
        receive(raw);
        delivered += 1;
      }
    });
  }

  function enqueue(message: unknown): void {
    if (detached || lost) return;
    if (queued.length >= limit) throw new DemoRpcError('overloaded', 'Demo response queue is full.');
    const raw = JSON.stringify(message);
    const bytes = encoder.encode(raw).length;
    if (bytes > maxMessageBytes) throw new DemoRpcError('too_large', 'Demo response exceeds its byte limit.');
    if (queuedBytes + bytes > maxQueueBytes) throw new DemoRpcError('overloaded', 'Demo response byte capacity reached.');
    queued.push(raw);
    queuedBytes += bytes;
    flush();
  }

  const transport: ControlTransport = {
    attach(next, onDisconnect, onReconnect) {
      generation += 1;
      detached = false;
      receive = next; disconnected = onDisconnect; reconnected = onReconnect;
      scheduled = false;
      flush();
    },
    detach() {
      generation += 1;
      detached = true;
      receive = undefined; disconnected = undefined; reconnected = undefined;
      scheduled = false;
      queued = [];
      queuedBytes = 0;
    },
    send(raw) {
      if (detached || lost) throw new DemoRpcError('transport', 'The demo transport is disconnected.');
      if (queued.length >= limit) throw new DemoRpcError('overloaded', 'Demo response queue is full.');
      if (queuedBytes > maxQueueBytes - maxMessageBytes) throw new DemoRpcError('overloaded', 'Demo response byte capacity reached.');
      let id: string | number | null = null;
      try {
        if (encoder.encode(raw).length > 1024 * 1024) throw new DemoRpcError('too_large', 'Demo request exceeds its byte limit.');
        const request = objectParams(JSON.parse(raw));
        if ((typeof request.id !== 'string' || request.id.length > 256) && (typeof request.id !== 'number' || !Number.isSafeInteger(request.id))) throw new DemoRpcError('invalid_request', 'A request ID is required.');
        id = request.id as string | number;
        if (request.jsonrpc !== '2.0' || typeof request.method !== 'string') throw new DemoRpcError('invalid_request', 'Expected a JSON-RPC 2.0 method.');
        const method = methods.get(request.method);
        if (!method) throw new DemoRpcError('method_not_found', `This demo does not support ${request.method}.`);
        const metadata = request.meta === undefined ? undefined : objectParams(request.meta) as RequestMetadata;
        const reply = method.invoke(request.params ?? {}, metadata);
        if (reply.meta !== undefined && !isResponseMetadata(reply.meta)) throw new DemoRpcError('protocol', 'Invalid simulated response metadata.');
        enqueue({ type: 'rpc-response', response: { jsonrpc: '2.0', id, result: reply.result, ...(reply.meta ? { meta: reply.meta } : {}) } });
      } catch (failure) {
        const error = failure instanceof DemoRpcError ? failure : new DemoRpcError('invalid_request', failure instanceof SyntaxError ? 'Malformed JSON request.' : 'The simulated operation failed.');
        enqueue({ type: 'rpc-response', response: { jsonrpc: '2.0', id, error: { code: error.code, message: error.message, ...(error.data === undefined ? {} : { data: error.data }) } } });
      }
    },
  };

  return {
    transport,
    emitTopic: (topic: string, data: unknown) => enqueue({ jsonrpc: '2.0', method: 'event', params: { topic, data } }),
    emitPush: (message: unknown) => enqueue(message),
    disconnect(reason = 'Simulated connection interrupted') {
      generation += 1; queued = []; queuedBytes = 0; scheduled = false; lost = true;
      disconnected?.(reason);
    },
    reconnect() { lost = false; reconnected?.(); },
    diagnostics: () => ({ queuedResponses: queued.length, queuedBytes, maxMessageBytes, maxQueueBytes, callbacks: receive ? 3 : 0, delivered, detached, lost, capacity: limit }),
  };
}
