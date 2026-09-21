/**
 * Reusable event-gateway WebSocket client: auth, subscription, heartbeat,
 * exponential-backoff reconnect, and `event.gap` reporting.
 *
 * Transport only: parsed envelopes flow to `onEvent` handlers; widgets map
 * them onto alerts in their controllers.
 */

import {
  DEFAULT_GATEWAY_HOST,
  DEFAULT_GATEWAY_PORT,
  DEFAULT_TOPICS,
  EVENT_GAP_TOPIC,
  gatewayWsUrl,
  HEARTBEAT_INTERVAL_MS,
  HEARTBEAT_MISSED_LIMIT,
} from './config.ts';
import { isDomainEventEnvelope, isGatewayControlMessage, parseEventGap } from './event-parser.ts';
import type { DomainEventEnvelope, EventGapData, GatewayControlMessage } from './event-types.ts';
import { createLogger } from './logger.ts';
import { computeReconnectDelay } from './reconnect.ts';

export type GatewayStatus =
  | 'idle'
  | 'connecting'
  | 'authenticating'
  | 'connected'
  | 'reconnecting'
  | 'auth-failed'
  | 'closed';

export type GatewayEventHandler = (envelope: DomainEventEnvelope) => void;
export type GatewayGapHandler = (gap: EventGapData) => void;
export type GatewayStatusHandler = (status: GatewayStatus) => void;

export interface GatewayClientOptions {
  host?: string;
  port?: number;
  /** Token value or a resolver; resolved fresh on every (re)connect. */
  token: string | (() => string | null);
  topics?: readonly string[];
  heartbeatMs?: number;
  reconnectRandom?: () => number;
  /** Injectable WebSocket for tests. Defaults to the global. */
  webSocketImpl?: typeof WebSocket;
}

type Timer = ReturnType<typeof setTimeout>;

const log = createLogger('gateway');

export class GatewayClient {
  private readonly host: string;
  private readonly port: number;
  private readonly resolveToken: () => string | null;
  private readonly topics: readonly string[];
  private readonly heartbeatMs: number;
  private readonly reconnectRandom: (() => number) | undefined;
  private readonly webSocketImpl: typeof WebSocket | undefined;

  private socket: WebSocket | null = null;
  private status: GatewayStatus = 'idle';
  private attempt = 0;
  private reconnectTimer: Timer | null = null;
  private heartbeatTimer: Timer | null = null;
  private lastFrameAt = 0;
  private intentionalClose = false;

  private readonly eventHandlers = new Set<GatewayEventHandler>();
  private readonly gapHandlers = new Set<GatewayGapHandler>();
  private readonly statusHandlers = new Set<GatewayStatusHandler>();
  private onlineHandler: (() => void) | null = null;

  constructor(options: GatewayClientOptions) {
    this.host = options.host ?? DEFAULT_GATEWAY_HOST;
    this.port = options.port ?? DEFAULT_GATEWAY_PORT;
    const token = options.token;
    this.resolveToken = typeof token === 'function' ? token : () => token;
    this.topics = options.topics ?? DEFAULT_TOPICS;
    this.heartbeatMs = options.heartbeatMs ?? HEARTBEAT_INTERVAL_MS;
    this.reconnectRandom = options.reconnectRandom;
    this.webSocketImpl =
      options.webSocketImpl ?? (typeof WebSocket === 'undefined' ? undefined : WebSocket);
  }

  get currentStatus(): GatewayStatus {
    return this.status;
  }

  get url(): string {
    return gatewayWsUrl(this.host, this.port);
  }

  onEvent(handler: GatewayEventHandler): () => void {
    this.eventHandlers.add(handler);
    return () => {
      this.eventHandlers.delete(handler);
    };
  }

  onGap(handler: GatewayGapHandler): () => void {
    this.gapHandlers.add(handler);
    return () => {
      this.gapHandlers.delete(handler);
    };
  }

  onStatus(handler: GatewayStatusHandler): () => void {
    this.statusHandlers.add(handler);
    return () => {
      this.statusHandlers.delete(handler);
    };
  }

  connect(): void {
    this.intentionalClose = false;
    this.clearReconnectTimer();
    if (this.socket !== null) return;
    const impl = this.webSocketImpl;
    if (!impl) {
      log.error('no WebSocket implementation available');
      return;
    }
    const token = this.resolveToken();
    if (!token) {
      log.warn('no gateway token available; staying idle until credentials exist');
      return;
    }
    this.setStatus(this.attempt === 0 ? 'connecting' : 'reconnecting');
    let socket: WebSocket;
    try {
      socket = new impl(this.url);
    } catch (error) {
      log.warn('websocket construction failed', error);
      this.scheduleReconnect();
      return;
    }
    this.socket = socket;
    socket.onopen = () => {
      this.lastFrameAt = Date.now();
      this.setStatus('authenticating');
      this.send({ type: 'auth', token });
    };
    socket.onmessage = (message: MessageEvent) => {
      this.handleFrame(typeof message.data === 'string' ? message.data : '');
    };
    socket.onerror = () => {
      // Errors are always followed by close; reconnect from there.
      log.debug('websocket error observed; awaiting close');
    };
    socket.onclose = () => {
      this.handleClose();
    };
    this.bindOnlineHandler();
  }

  disconnect(): void {
    this.intentionalClose = true;
    this.clearReconnectTimer();
    this.clearHeartbeat();
    this.unbindOnlineHandler();
    const socket = this.socket;
    this.socket = null;
    if (socket) {
      socket.onopen = null;
      socket.onmessage = null;
      socket.onerror = null;
      socket.onclose = null;
      try {
        socket.close();
      } catch {
        // Already closed; ignore.
      }
    }
    this.attempt = 0;
    this.setStatus('closed');
  }

  /** Retries a connection when the page regains credentials or network. */
  retryNow(): void {
    if (this.socket !== null || this.intentionalClose) return;
    this.attempt = 0;
    this.connect();
  }

  private setStatus(status: GatewayStatus): void {
    if (this.status === status) return;
    this.status = status;
    for (const handler of this.statusHandlers) {
      try {
        handler(status);
      } catch (error) {
        log.warn('status handler threw', error);
      }
    }
  }

  private send(payload: unknown): void {
    const socket = this.socket;
    if (!socket || socket.readyState !== 1) return;
    try {
      socket.send(JSON.stringify(payload));
    } catch (error) {
      log.warn('failed to send control message', error);
    }
  }

  private handleFrame(text: string): void {
    if (!text) return;
    this.lastFrameAt = Date.now();
    let parsed: unknown;
    try {
      parsed = JSON.parse(text);
    } catch {
      log.warn('ignoring invalid JSON frame');
      return;
    }
    if (isGatewayControlMessage(parsed)) {
      this.handleControl(parsed);
      return;
    }
    if (!isDomainEventEnvelope(parsed)) {
      log.warn('ignoring malformed gateway frame');
      return;
    }
    if (parsed.topic === EVENT_GAP_TOPIC) {
      const gap = parseEventGap(parsed);
      if (gap) {
        log.warn(`event gap: lost ${gap.lost} message(s); continuing with new events`);
        for (const handler of this.gapHandlers) {
          try {
            handler(gap);
          } catch (error) {
            log.warn('gap handler threw', error);
          }
        }
      } else {
        log.warn('ignoring malformed event.gap frame');
      }
      return;
    }
    for (const handler of this.eventHandlers) {
      try {
        handler(parsed);
      } catch (error) {
        log.warn('event handler threw; continuing with next frame', error);
      }
    }
  }

  private handleControl(message: GatewayControlMessage): void {
    switch (message.type) {
      case 'authenticated':
        this.send({ type: 'subscribe', topics: [...this.topics] });
        break;
      case 'subscribed':
        this.attempt = 0;
        this.setStatus('connected');
        this.startHeartbeat();
        break;
      case 'pong':
        break;
      case 'error': {
        if (this.status === 'authenticating') {
          // A wrong token never heals by retrying; park until fixed.
          log.error('gateway authentication failed; not retrying automatically');
          this.clearReconnectTimer();
          this.closeSocket();
          this.attempt = 0;
          this.setStatus('auth-failed');
        } else {
          log.warn('gateway control error', message.error);
        }
        break;
      }
    }
  }

  private handleClose(): void {
    this.closeSocket();
    this.clearHeartbeat();
    if (this.intentionalClose) return;
    if (this.status === 'auth-failed') return;
    this.scheduleReconnect();
  }

  private closeSocket(): void {
    const socket = this.socket;
    this.socket = null;
    if (socket) {
      socket.onopen = null;
      socket.onmessage = null;
      socket.onerror = null;
      socket.onclose = null;
      if (socket.readyState === 1 || socket.readyState === 0) {
        try {
          socket.close();
        } catch {
          // Ignore close errors during teardown.
        }
      }
    }
  }

  private scheduleReconnect(): void {
    if (this.intentionalClose || this.reconnectTimer !== null) return;
    this.setStatus('reconnecting');
    const delay = computeReconnectDelay(this.attempt, { random: this.reconnectRandom });
    this.attempt += 1;
    log.info(`reconnecting in ${delay}ms (attempt ${this.attempt})`);
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = null;
      this.connect();
    }, delay);
  }

  private clearReconnectTimer(): void {
    if (this.reconnectTimer !== null) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }

  private startHeartbeat(): void {
    this.clearHeartbeat();
    this.lastFrameAt = Date.now();
    this.heartbeatTimer = setInterval(() => {
      const silentFor = Date.now() - this.lastFrameAt;
      if (silentFor > this.heartbeatMs * HEARTBEAT_MISSED_LIMIT) {
        log.warn('gateway silent beyond heartbeat budget; reconnecting');
        this.closeSocket();
        this.clearHeartbeat();
        this.scheduleReconnect();
        return;
      }
      this.send({ type: 'ping' });
    }, this.heartbeatMs);
  }

  private clearHeartbeat(): void {
    if (this.heartbeatTimer !== null) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  private bindOnlineHandler(): void {
    if (typeof window === 'undefined' || this.onlineHandler) return;
    const handler = (): void => {
      if (this.socket === null && !this.intentionalClose && this.status !== 'auth-failed') {
        this.clearReconnectTimer();
        this.attempt = 0;
        this.connect();
      }
    };
    this.onlineHandler = handler;
    window.addEventListener('online', handler);
  }

  private unbindOnlineHandler(): void {
    if (typeof window === 'undefined' || !this.onlineHandler) return;
    window.removeEventListener('online', this.onlineHandler);
    this.onlineHandler = null;
  }
}
