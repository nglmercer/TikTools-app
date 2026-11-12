import type { ControlTransport } from './control-transport.ts';
import { ControlCallError, type ControlCallOptions, type RpcParams, type ResponseMetadata } from './control-contracts.ts';
import { isResponseMetadata, type RpcResponseWire } from './control-wire.ts';

type PendingCall = {
  resolve: (result: unknown, meta?: ResponseMetadata) => void;
  reject: (error: ControlCallError) => void;
  timer: ReturnType<typeof setTimeout>;
  cleanupAbort: () => void;
};

const DEFAULT_TIMEOUT_MS = 150_000;
const MAX_PENDING_CALLS = 128;
// Mirrors MAX_REQUEST_BYTES in crates/tiktools-control-api/src/transport.rs

const MAX_WEBVIEW_REQUEST_BYTES = 1024 * 1024;

const textEncoder = new TextEncoder();

/** Pending calls, deadlines and cancellation share one bounded queue per client. */
export function createControlRequests(context: {
  transport?: ControlTransport;
  timeoutMs?: number;
  generation: () => number | null;
  detached: () => boolean;
  reportTransportError: (message: string) => void;
  handledResponse: () => void;
}) {
  const transport = context.transport;
  const timeoutMs = context.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  let nextId = 0;
  const requestPrefix = globalThis.crypto?.randomUUID?.() ??
    `${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`;
  const pending = new Map<number | string, PendingCall>();
  let overloaded = 0;
  const call = <T = unknown>(method: string, params?: RpcParams, options?: ControlCallOptions): Promise<T> => {
    return new Promise<T>((resolve, reject) => {
      const signal = options?.signal;
      if (options?.timeoutMs !== undefined && (!Number.isInteger(options.timeoutMs) || options.timeoutMs < 1 || options.timeoutMs > 300_000)) {
        reject(new ControlCallError('client', 'invalid control request timeout'));
        return;
      }
      if (signal?.aborted) { reject(new ControlCallError('cancelled', 'control request owner disposed')); return; }
      if (context.detached()) {
        reject(new ControlCallError('transport', 'control client detached'));
        return;
      }
      if (pending.size >= MAX_PENDING_CALLS) {
        overloaded += 1;
        const error = new ControlCallError('overloaded', 'control client pending request limit reached');
        context.reportTransportError(error.message);
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
          ...(options?.meta === undefined ? {} : { meta: options.meta }),
          ...(context.generation() === null ? {} : { pageGeneration: context.generation() }),
        });
      } catch (error) {
        reject(
          new ControlCallError('transport', `could not serialize ${method}: ${String(error)}`),
        );
        return;
      }
      const requestLimit = method === 'automation.library.submit' ? 8 * 1024 * 1024 + 64 * 1024 : MAX_WEBVIEW_REQUEST_BYTES;
      if (textEncoder.encode(payload).length > requestLimit) {
        const error = new ControlCallError(
          'too_large',
          `request ${method} exceeds the ${requestLimit}-byte limit`,
        );
        context.reportTransportError(error.message);
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
        context.reportTransportError(error.message);
        reject(error);
      }, options?.timeoutMs ?? timeoutMs);
      pending.set(id, {
        resolve: (result, meta) => {
          try { options?.onMetadata?.(meta); }
          catch { reject(new ControlCallError('client', 'could not process response metadata')); return; }
          resolve(result as T);
        },
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
  };

  function settle(response: RpcResponseWire): void {
    if (response.id === undefined || response.id === null) return;
    const call = pending.get(response.id);
    if (!call) return;
    context.handledResponse();
    pending.delete(response.id);
    clearTimeout(call.timer);
    call.cleanupAbort();
    if (response.error !== undefined && response.error !== null) {
      const code = typeof response.error.code === 'string' ? response.error.code : 'error';
      const message =
        typeof response.error.message === 'string' ? response.error.message : 'request failed';
      call.reject(new ControlCallError(code, message, response.error.data));
    } else {
      const metadata = response.meta;
      if (metadata !== undefined && metadata !== null && !isResponseMetadata(metadata)) {
        call.reject(new ControlCallError('protocol', 'invalid response metadata'));
        return;
      }
      call.resolve(response.result ?? null, metadata ?? undefined);
    }
  }

  function rejectAll(reason: string): void {
    for (const call of pending.values()) {
      clearTimeout(call.timer);
      call.cleanupAbort();
      call.reject(new ControlCallError('transport', reason));
    }
    pending.clear();
  }
  return { call, settle, rejectAll, diagnostics: () => ({ pending: pending.size, capacity: MAX_PENDING_CALLS, overloaded }) };
}
