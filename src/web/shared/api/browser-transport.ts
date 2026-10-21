import type { ControlTransport } from './control-transport.ts';

/** No queued/replayed requests: accepted mutations may finish after disconnect. */
export function createBrowserTransport(options: {
  url: string;
  socket?: (url: string) => WebSocket;
  status?: (status: string) => void;
}): ControlTransport & { connect(): Promise<void> } {
  const makeSocket = options.socket ?? ((url: string) => new WebSocket(url));
  let socket: WebSocket | undefined;
  let stopped = false;
  let retry: ReturnType<typeof setTimeout> | undefined;
  let failures = 0;
  let wasConnected = false;
  const initialMessages: string[] = [];
  let initialBytes = 0;
  let initialGap = false;
  let receive: ((raw: string) => void) | undefined;
  let disconnected: ((reason: string) => void) | undefined;
  let reconnected: (() => void) | undefined;

  function connect(): Promise<void> {
    stopped = false;
    options.status?.(wasConnected ? 'Reconnecting to TikTools…' : 'Connecting to TikTools…');
    return new Promise((resolve, reject) => {
      const current = makeSocket(options.url);
      socket = current;
      const deadline = setTimeout(() => current.close(), 10_000);
      current.onopen = () => {
        clearTimeout(deadline);
        failures = 0;
        options.status?.('Connected');
        if (wasConnected) reconnected?.();
        wasConnected = true;
        resolve();
      };
      current.onmessage = (event) => {
        if (typeof event.data !== 'string') { current.close(); return; }
        if (new TextEncoder().encode(event.data).length > 1024 * 1024) { current.close(); return; }
        if (receive) receive(event.data);
        else {
          const size = new TextEncoder().encode(event.data).length;
          while (initialMessages.length >= 256 || initialBytes + size > 1024 * 1024) {
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
        options.status?.('Disconnected. Reconnecting… If pairing expired, reload to pair again.');
        disconnected?.('connection lost; accepted operations may still complete');
        reject(new Error('Local host unavailable or pairing expired'));
        const delay = Math.min(30_000, 500 * 2 ** Math.min(failures++, 6));
        retry = setTimeout(() => { retry = undefined; void connect().catch(() => {}); }, delay);
      };
      current.onerror = () => current.close();
    });
  }
  return {
    connect,
    send(message) {
      if (socket?.readyState !== 1) throw new Error('local browser connection is unavailable');
      if (socket.bufferedAmount + new TextEncoder().encode(message).length > 1024 * 1024) {
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
      socket?.close();
      socket = undefined;
      initialMessages.length = 0; initialBytes = 0; initialGap = false;
      receive = undefined;
      disconnected = undefined;
      reconnected = undefined;
    },
  };
}
