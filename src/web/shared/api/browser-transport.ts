import type { ControlTransport } from './control-transport.ts';

// Local Owner source reviews may be larger than incoming request frames.
// Remote controller transports retain their independent negotiated limits.
const OWNER_RESPONSE_BYTES = 8 * 1024 * 1024;
const REQUEST_QUEUE_BYTES = 1024 * 1024;
const SESSION_TOUCH = '/__local/session/touch';

export type LocalSessionState = 'ok' | 'expired' | 'unavailable';

/** A failed session probe, so callers never mistake auth loss for a blip. */
export class LocalSessionError extends Error {
  constructor(readonly kind: Exclude<LocalSessionState, 'ok'>) {
    super(kind === 'expired' ? 'Local browser session expired' : 'Local host unavailable');
    this.name = 'LocalSessionError';
  }
}

/**
 * Validates the cookie and slides the adapter's idle window. Same-origin POST so
 * the adapter can both check Origin and reissue the cookie. A network failure
 * is `unavailable`, never `expired`: the session may still be valid.
 */
export async function touchLocalSession(): Promise<LocalSessionState> {
  try {
    const response = await fetch(SESSION_TOUCH, {
      method: 'POST', credentials: 'same-origin', cache: 'no-store', redirect: 'error',
    });
    if (response.ok) return 'ok';
    if (response.status === 401 || response.status === 403) return 'expired';
    return 'unavailable';
  } catch {
    return 'unavailable';
  }
}

/** No queued/replayed requests: accepted mutations may finish after disconnect. */
export function createBrowserTransport(options: {
  url: string;
  socket?: (url: string) => WebSocket;
  status?: (status: string) => void;
  /** When false, the owner retries; the transport only reports the loss. */
  autoReconnect?: boolean;
  /** Preflight that distinguishes an expired cookie from an unreachable host. */
  session?: { probe: () => Promise<LocalSessionState> };
  sessionState?: (state: LocalSessionState | 'reconnecting') => void;
}): ControlTransport & { connect(): Promise<void> } {
  const makeSocket = options.socket ?? ((url: string) => new WebSocket(url));
  let socket: WebSocket | undefined;
  let stopped = false;
  let retry: ReturnType<typeof setTimeout> | undefined;
  let failures = 0;
  let cancelOpening: (() => void) | undefined;
  let wasConnected = false;
  const initialMessages: string[] = [];
  let initialBytes = 0;
  let initialGap = false;
  let receive: ((raw: string) => void) | undefined;
  let disconnected: ((reason: string) => void) | undefined;
  let reconnected: (() => void) | undefined;

  function connect(): Promise<void> {
    clearTimeout(retry); retry = undefined;
    const previous = socket; socket = undefined;
    if (previous?.readyState === 1) {
      initialMessages.length = 0; initialBytes = 0; initialGap = true;
      disconnected?.('connection replaced; accepted operations may still complete');
    }
    cancelOpening?.(); cancelOpening = undefined;
    previous?.close();
    stopped = false;
    options.status?.(wasConnected ? 'Reconnecting to TikTools…' : 'Connecting to TikTools…');
    const open = (): Promise<void> => new Promise((resolve, reject) => {
      const current = makeSocket(options.url);
      socket = current;
      const deadline = setTimeout(() => current.close(), 10_000);
      cancelOpening = () => {
        clearTimeout(deadline);
        reject(new Error('Local browser connection cancelled'));
      };
      current.onopen = () => {
        if (socket !== current || stopped) return;
        clearTimeout(deadline);
        cancelOpening = undefined;
        failures = 0;
        options.sessionState?.('ok');
        options.status?.('Connected');
        if (wasConnected) reconnected?.();
        wasConnected = true;
        resolve();
      };
      current.onmessage = (event) => {
        if (socket !== current || stopped) return;
        if (typeof event.data !== 'string') { current.close(); return; }
        const size = new TextEncoder().encode(event.data).length;
        if (size > OWNER_RESPONSE_BYTES) { current.close(); return; }
        if (receive) receive(event.data);
        else {
          while (initialMessages.length >= 256 || initialBytes + size > OWNER_RESPONSE_BYTES) {
            const dropped = initialMessages.shift();
            if (dropped === undefined) break;
            initialBytes -= new TextEncoder().encode(dropped).length;
            initialGap = true;
          }
          initialMessages.push(event.data); initialBytes += size;
        }
      };
      current.onclose = () => {
        clearTimeout(deadline);
        if (socket !== current || stopped) return;
        socket = undefined;
        cancelOpening = undefined;
        initialMessages.length = 0; initialBytes = 0;
        if (wasConnected) initialGap = true;
        options.sessionState?.('reconnecting');
        options.status?.('Disconnected. Reconnecting…');
        disconnected?.('connection lost; accepted operations may still complete');
        reject(new Error('Local host unavailable or pairing expired'));
        if (options.autoReconnect === false) return;
        const delay = Math.min(30_000, 500 * 2 ** Math.min(failures++, 6));
        retry = setTimeout(() => { retry = undefined; void connect().catch(() => {}); }, delay);
      };
      current.onerror = () => { if (socket === current && !stopped) current.close(); };
    });
    if (!options.session) return open();
    return options.session.probe().then((state) => {
      if (stopped) throw new Error('Local browser connection cancelled');
      if (state === 'ok') return open();
      options.sessionState?.(state);
      options.status?.(state === 'expired' ? 'Pairing expired. Pair again to continue.' : 'Local host unavailable.');
      throw new LocalSessionError(state);
    });
  }
  return {
    connect,
    send(message) {
      if (socket?.readyState !== 1) throw new Error('local browser connection is unavailable');
      const bytes = new TextEncoder().encode(message).length;
      let limit = REQUEST_QUEUE_BYTES;
      if (bytes > REQUEST_QUEUE_BYTES && bytes <= 8 * 1024 * 1024 + 64 * 1024) {
        try { if ((JSON.parse(message) as { method?: unknown }).method === 'automation.library.submit') limit = 8 * 1024 * 1024 + 64 * 1024; } catch { /* malformed frames retain the ordinary ceiling */ }
      }
      if (socket.bufferedAmount + bytes > limit) {
        throw new Error('local browser send queue is full');
      }
      socket.send(message);
    },
    attach(onMessage, onDisconnect, onReconnect) {
      receive = onMessage;
      disconnected = onDisconnect;
      reconnected = onReconnect;
      for (const message of initialMessages.splice(0)) onMessage(message);
      initialBytes = 0;
      if (initialGap) { initialGap = false; onReconnect(); }
      if (!socket && !retry) void connect().catch(() => {});
    },
    detach() {
      stopped = true;
      clearTimeout(retry);
      retry = undefined;
      const previous = socket; socket = undefined;
      cancelOpening?.(); cancelOpening = undefined;
      previous?.close();
      initialMessages.length = 0; initialBytes = 0; initialGap = false;
      receive = undefined;
      disconnected = undefined;
      reconnected = undefined;
    },
  };
}
