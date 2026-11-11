import type { ControlTransport } from './control-transport.ts';

const encoder = new TextEncoder();
const MAX_FRAME = 1024 * 1024;
const OPEN_TIMEOUT_MS = 10_000;

type Identity = { principalId: string; sessionId: string; grantRevision: string };
export type RemoteBrowserHello = Identity & {
  hostId: string;
  hostInstanceId: string;
  connectionId: string;
  frameBytes: number;
  paramsBytes: number;
  requestsPerSecond: number;
  requestBurst: number;
  pendingCalls: number;
  serverTime: number;
  capabilities: readonly string[];
  subscriptionDomains: readonly string[];
  features: Readonly<Record<string, boolean>>;
  resourceRestrictions: Readonly<Record<string, readonly string[]>>;
};
export interface RemoteBrowserTransport extends ControlTransport {
  connect(): Promise<RemoteBrowserHello>;
  disconnect(): void;
  currentHello(): RemoteBrowserHello | undefined;
  verifySession(): Promise<boolean>;
  /** Raw envelope, including subscription, resource and sequence metadata. */
  onReliableEvent(handler: (event: unknown) => void): () => void;
  /** Also fires on explicit detach so private projections can be cleared. */
  onConnectionLost(handler: (reason: string) => void): () => void;
}

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function id(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= 128 &&
    Array.from(value).every((character) => {
      const code = character.codePointAt(0)!;
      return code >= 32 && code !== 127;
    });
}
function revision(value: unknown): value is string {
  return typeof value === 'string' && /^(0|[1-9][0-9]{0,18})$/.test(value);
}
function identity(value: unknown): Identity {
  if (!record(value) || value['clientKind'] !== 'browser' || !id(value['principalId']) ||
      !id(value['sessionId']) || !revision(value['grantRevision'])) throw new Error('invalid browser session');
  return { principalId: value['principalId'], sessionId: value['sessionId'], grantRevision: value['grantRevision'] };
}
function hello(value: unknown, expected: Identity): RemoteBrowserHello {
  if (!record(value) || value['protocolVersion'] !== 1 || value['principalId'] !== expected.principalId ||
      value['sessionId'] !== expected.sessionId || value['grantRevision'] !== expected.grantRevision ||
      !id(value['hostId']) || !id(value['hostInstanceId']) || !id(value['connectionId']) ||
      !record(value['limits'])) throw new Error('invalid remote negotiation');
  const limits = value['limits'];
  const frame = limits['frameBytes'];
  const params = limits['paramsBytes'];
  const rate = limits['requestsPerSecond'];
  const burst = limits['requestBurst'];
  const pending = limits['pendingCalls'];
  if (typeof frame !== 'number' || !Number.isInteger(frame) || frame < 4096 || frame > MAX_FRAME ||
      typeof params !== 'number' || !Number.isInteger(params) || params < 1024 || params > Math.min(frame, 256 * 1024) ||
      typeof rate !== 'number' || !Number.isInteger(rate) || rate < 1 || rate > 30 ||
      typeof burst !== 'number' || !Number.isInteger(burst) || burst < rate || burst > 60 ||
      typeof pending !== 'number' || !Number.isInteger(pending) || pending < 1 || pending > 32) {
    throw new Error('invalid remote limits');
  }
  const capabilities = value['capabilities'];
  const features = value['features'];
  const restrictions = value['resourceRestrictions'];
  const domains = value['subscriptionDomains'];
  const serverTime = value['serverTime'];
  if (!Array.isArray(capabilities) || capabilities.length > 128 || !capabilities.every(id) ||
      new Set(capabilities).size !== capabilities.length ||
      !Array.isArray(domains) || domains.length > 128 || !domains.every(id) || new Set(domains).size !== domains.length ||
      typeof serverTime !== 'number' || !Number.isSafeInteger(serverTime) || serverTime < 0 ||
      !record(features) || Object.keys(features).length > 128 ||
      !Object.entries(features).every(([key, enabled]) => id(key) && typeof enabled === 'boolean') ||
      !record(restrictions) || Object.keys(restrictions).length > 128 ||
      !Object.entries(restrictions).every(([key, ids]) => id(key) && Array.isArray(ids) && ids.length <= 4096 && ids.every(id))) {
    throw new Error('invalid remote permissions');
  }
  const frozenRestrictions = Object.freeze(Object.fromEntries(Object.entries(restrictions)
    .map(([key, ids]) => [key, Object.freeze([...(ids as string[])])])));
  return Object.freeze({ ...expected, hostId: value['hostId'], hostInstanceId: value['hostInstanceId'],
    connectionId: value['connectionId'], frameBytes: frame, paramsBytes: params,
    requestsPerSecond: rate, requestBurst: burst, pendingCalls: pending, serverTime,
    capabilities: Object.freeze([...capabilities] as string[]),
    subscriptionDomains: Object.freeze([...domains] as string[]),
    features: Object.freeze({ ...features } as Record<string, boolean>), resourceRestrictions: frozenRestrictions });
}

/** Session outcomes a browser can act on: renew or pair again. */
export type RemoteSessionState = 'ok' | 'expired' | 'unavailable';

/** A classified session failure, never a bare network error. */
export class RemoteSessionError extends Error {
  constructor(readonly kind: Exclude<RemoteSessionState, 'ok'>, message = 'Remote browser session is unavailable') {
    super(message);
    this.name = 'RemoteSessionError';
  }
}

/**
 * Same-origin browser session transport for the remote gateway. It never
 * stores credentials or queues requests. A reconnect obtains a new ticket and
 * requires the caller to rebuild its authorized view before issuing writes.
 * This is transport plumbing; the native controller bridge is not available
 * in a remote browser and needs a separate browser controller adapter.
 */
export function createRemoteBrowserTransport(options: {
  fetch?: typeof fetch;
  socket?: (url: string) => WebSocket;
  /** Observes session health so the UI can recover without a manual reload. */
  onSessionState?: (state: RemoteSessionState | 'reconnecting') => void;
} = {}): RemoteBrowserTransport {
  const fetcher = options.fetch ?? globalThis.fetch.bind(globalThis);
  const makeSocket = options.socket ?? ((url: string) => new WebSocket(url));
  let generation = 0;
  let abort: AbortController | undefined;
  let socket: WebSocket | undefined;
  let ready: RemoteBrowserHello | undefined;
  let opening = false;
  let everConnected = false;
  let receive: ((message: string) => void) | undefined;
  let disconnected: ((reason: string) => void) | undefined;
  let reconnected: (() => void) | undefined;
  let missedDisconnect: string | undefined;
  let verification: { epoch: number; promise: Promise<boolean> } | undefined;
  let sessionTimer: ReturnType<typeof setInterval> | undefined;
  let channel: BroadcastChannel | undefined;
  try {
    if (typeof BroadcastChannel !== 'undefined') {
      channel = new BroadcastChannel('tiktools-remote-session-v1');
      channel.onmessage = (event) => { if (event.data === 'session-changed' && ready) void verifySession(); };
    }
  } catch { /* periodic verification remains active */ }
  const reliableListeners = new Set<(event: unknown) => void>();
  const lossListeners = new Set<(reason: string) => void>();
  const buffered: string[] = [];
  let bufferedBytes = 0;
  const current = (epoch: number): void => {
    if (epoch !== generation || abort?.signal.aborted) throw new Error('remote connection cancelled');
  };
  function close(reason: string, notify: boolean): void {
    ++generation;
    abort?.abort(); abort = undefined;
    clearInterval(sessionTimer); sessionTimer = undefined;
    stopSessionSignals();
    verification = undefined;
    const previous = socket; socket = undefined;
    const wasReady = ready !== undefined;
    ready = undefined;
    opening = false;
    buffered.length = 0; bufferedBytes = 0;
    previous?.close();
    if (wasReady) for (const listener of lossListeners) {
      try { listener(reason); } catch { /* other views must still be invalidated */ }
    }
    if (notify && wasReady) {
      if (disconnected) disconnected(reason);
      else missedDisconnect = reason;
    }
  }
  const onVisibility = (): void => { if (document.visibilityState === 'visible' && ready) void verifySession(); };
  const onOnline = (): void => { if (ready) void verifySession(); };
  function stopSessionSignals(): void {
    if (typeof document !== 'undefined') document.removeEventListener('visibilitychange', onVisibility);
    if (typeof window !== 'undefined') window.removeEventListener('online', onOnline);
  }
  function startSessionSignals(): void {
    clearInterval(sessionTimer);
    // A 15 s keepalive, not a liveness probe: the socket heartbeat reports
    // transport health, and each check also slides the session's idle window.
    sessionTimer = setInterval(() => {
      if (document.visibilityState !== 'hidden') void verifySession();
    }, 15_000);
    document.addEventListener('visibilitychange', onVisibility);
    window.addEventListener('online', onOnline);
  }
  async function verifySession(): Promise<boolean> {
    const expected = ready;
    const controller = abort;
    const epoch = generation;
    if (!expected || !controller || controller.signal.aborted) return false;
    if (verification?.epoch === epoch) return verification.promise;
    const promise = (async (): Promise<boolean> => {
      const check = new AbortController();
      const cancel = (): void => check.abort();
      controller.signal.addEventListener('abort', cancel, { once: true });
      const deadline = setTimeout(cancel, 3000);
      try {
        const response = await fetcher('/remote/v1/session', {
          credentials: 'same-origin', mode: 'same-origin', redirect: 'error', cache: 'no-store', signal: check.signal,
        });
        if (!response.ok) {
          // Only an auth answer invalidates the connection. A 5xx or a proxy
          // hiccup would otherwise log out a browser whose cookie is fine.
          if (response.status === 401 || response.status === 403) {
            throw new RemoteSessionError('expired', 'remote browser session expired');
          }
          return false;
        }
        const value: unknown = await response.json();
        if (!record(value)) throw new RemoteSessionError('unavailable', 'invalid browser session');
        const observed = identity(value['identity']);
        if (epoch !== generation || ready !== expected) return false;
        if (observed.principalId !== expected.principalId || observed.sessionId !== expected.sessionId || observed.grantRevision !== expected.grantRevision) {
          throw new RemoteSessionError('expired', 'remote session changed');
        }
        return true;
      } catch (error) {
        if (error instanceof RemoteSessionError && error.kind === 'expired' && epoch === generation && ready === expected) {
          options.onSessionState?.('expired');
          close('remote browser session expired', true);
        }
        // Network or timeout: the socket may still be healthy. Never drop a
        // session because one request was slow; the next tick retries.
        return false;
      } finally {
        clearTimeout(deadline);
        controller.signal.removeEventListener('abort', cancel);
        if (verification?.epoch === epoch) verification = undefined;
      }
    })();
    verification = { epoch, promise };
    return promise;
  }
  async function connect(): Promise<RemoteBrowserHello> {
    if (opening || ready) throw new Error('remote connection already active');
    if (location.protocol !== 'https:') throw new Error('remote browser requires HTTPS');
    opening = true;
    const epoch = ++generation;
    const controller = new AbortController(); abort = controller;
    const timer = setTimeout(() => controller.abort(), OPEN_TIMEOUT_MS);
    try {
      const response = await fetcher('/remote/v1/session', { credentials: 'same-origin', mode: 'same-origin', redirect: 'error', cache: 'no-store', signal: controller.signal });
      current(epoch);
      if (!response.ok) {
        throw new RemoteSessionError(
          response.status === 401 || response.status === 403 ? 'expired' : 'unavailable',
          'remote browser session unavailable',
        );
      }
      const session: unknown = await response.json(); current(epoch);
      if (!record(session) || !id(session['csrfToken'])) throw new Error('invalid browser session');
      const expected = identity(session['identity']);
      const ticket = await fetcher('/remote/v1/ws-ticket', {
        method: 'POST', credentials: 'same-origin', mode: 'same-origin', redirect: 'error', cache: 'no-store', signal: controller.signal,
        headers: { 'x-csrf-token': session['csrfToken'] },
      });
      current(epoch);
      if (!ticket.ok) throw new Error('remote socket ticket unavailable');
      const endpoint = new URL('/remote/v1/ws', location.origin);
      endpoint.protocol = 'wss:';
      const result = await new Promise<RemoteBrowserHello>((resolve, reject) => {
        const ws = makeSocket(endpoint.toString()); socket = ws;
        const helloId = `hello-${crypto.randomUUID()}`;
        let settled = false;
        const fail = (message: string): void => {
          if (!settled) { settled = true; reject(new Error(message)); }
        };
        const cancelled = (): void => { fail('remote connection cancelled'); ws.close(); };
        controller.signal.addEventListener('abort', cancelled, { once: true });
        ws.onopen = () => {
          if (epoch !== generation || controller.signal.aborted) { cancelled(); return; }
          ws.send(JSON.stringify({ jsonrpc: '2.0', id: helloId, method: 'collaboration.hello',
            params: { protocolVersion: 1, clientVersion: '4.1.0', clientKind: 'browser' } }));
        };
        ws.onmessage = (event) => {
          if (epoch !== generation || controller.signal.aborted || socket !== ws) return;
          if (typeof event.data !== 'string' || encoder.encode(event.data).length > (ready?.frameBytes ?? MAX_FRAME)) {
            fail('invalid remote frame'); close('invalid remote frame', true); return;
          }
          let value: unknown;
          try { value = JSON.parse(event.data) as unknown; } catch { fail('invalid remote frame'); close('invalid remote frame', true); return; }
          if (!record(value) || value['jsonrpc'] !== '2.0') { fail('invalid remote frame'); close('invalid remote frame', true); return; }
          if (!settled) {
            try {
              if (value['id'] !== helloId || value['error'] !== undefined) throw new Error('remote negotiation failed');
              const negotiated = hello(value['result'], expected);
              settled = true;
              ready = negotiated;
              startSessionSignals();
              controller.signal.removeEventListener('abort', cancelled);
              resolve(negotiated);
            } catch { fail('remote negotiation failed'); close('remote negotiation failed', true); }
            return;
          }
          if (!ready) return;
          const isResponse = typeof value['id'] === 'string' || typeof value['id'] === 'number';
          const isEvent = value['method'] === 'event' || value['method'] === 'event.gap';
          if (!isResponse && !isEvent) { close('invalid remote frame', true); return; }
          if (value['method'] === 'event') {
            for (const listener of reliableListeners) {
              try { listener(value['params']); }
              catch { close('remote event handler failed', true); return; }
              if (!ready || epoch !== generation) return;
            }
          }
          const message = isResponse ? JSON.stringify({ type: 'rpc-response', response: value }) : event.data;
          if (receive) receive(message);
          else {
            const bytes = encoder.encode(message).length;
            if (buffered.length >= 128 || bufferedBytes + bytes > MAX_FRAME) { close('remote event buffer overflow', true); return; }
            buffered.push(message); bufferedBytes += bytes;
          }
          // A gap invalidates the authorized projection. Do not leave a
          // connection usable for commands until a caller has recovered it.
          if (value['method'] === 'event.gap') close('remote event gap requires recovery', true);
        };
        ws.onclose = () => {
          controller.signal.removeEventListener('abort', cancelled);
          fail('remote connection closed');
          if (epoch === generation && socket === ws) {
            options.onSessionState?.('reconnecting');
            close('remote connection lost; accepted operations may still complete', true);
          }
        };
        ws.onerror = () => { fail('remote connection failed'); ws.close(); };
      });
      current(epoch);
      options.onSessionState?.('ok');
      if (everConnected) reconnected?.();
      everConnected = true;
      return result;
    } catch (error) {
      if (epoch === generation) close('remote connection failed', false);
      throw error;
    } finally {
      clearTimeout(timer);
      if (epoch === generation) opening = false;
    }
  }
  return {
    connect,
    disconnect() { close('remote browser disconnected', true); },
    currentHello() { return ready; },
    verifySession,
    onReliableEvent(handler) {
      reliableListeners.add(handler);
      return () => { reliableListeners.delete(handler); };
    },
    onConnectionLost(handler) {
      lossListeners.add(handler);
      return () => { lossListeners.delete(handler); };
    },
    send(message) {
      if (!ready || socket?.readyState !== 1) throw new Error('remote browser connection is unavailable');
      let request: unknown;
      try { request = JSON.parse(message) as unknown; } catch { throw new Error('invalid remote request'); }
      if (!record(request) || request['jsonrpc'] !== '2.0' || !id(request['id']) ||
          !id(request['method']) || request['method'] === 'collaboration.hello' || !record(request['params'])) throw new Error('invalid remote request');
      const bytes = encoder.encode(message).length;
      if (bytes > ready.frameBytes || encoder.encode(JSON.stringify(request['params'])).length > ready.paramsBytes ||
          socket.bufferedAmount + bytes > ready.frameBytes) throw new Error('remote request exceeds negotiated limits');
      socket.send(message);
    },
    attach(onMessage, onDisconnect, onReconnect) {
      receive = onMessage; disconnected = onDisconnect; reconnected = onReconnect;
      for (const message of buffered.splice(0)) onMessage(message);
      bufferedBytes = 0;
      if (missedDisconnect) { const reason = missedDisconnect; missedDisconnect = undefined; onDisconnect(reason); }
    },
    detach() {
      close('remote browser detached', false);
      receive = undefined; disconnected = undefined; reconnected = undefined;
      missedDisconnect = undefined;
      reliableListeners.clear(); lossListeners.clear();
      channel?.close(); channel = undefined;
    },
  };
}
