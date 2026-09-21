/**
 * Shared widget bootstrap: credentials, gateway client, status fan-out, and
 * test-envelope injection. Both alert widgets build on this so no WebSocket
 * logic lives inside Vue components.
 */

import { DEFAULT_TOPICS } from './config.ts';
import { CredentialProvider } from './credentials.ts';
import type { DomainEventEnvelope, EventGapData } from './event-types.ts';
import { GatewayClient, type GatewayStatus } from './gateway-client.ts';
import { createLogger } from './logger.ts';

const log = createLogger('runtime');

export interface WidgetRuntimeOptions {
  topics?: readonly string[];
  heartbeatMs?: number;
  credentials?: CredentialProvider;
  onEnvelope: (envelope: DomainEventEnvelope) => void;
  onGap?: (gap: EventGapData) => void;
  onStatus?: (status: GatewayStatus) => void;
}

export interface WidgetRuntime {
  readonly client: GatewayClient;
  readonly credentials: CredentialProvider;
  readonly hasToken: boolean;
  status: GatewayStatus;
  start: () => void;
  stop: () => void;
  /** Feeds a test envelope through the same path as gateway frames. */
  injectTestEnvelope: (envelope: DomainEventEnvelope) => void;
}

export function createWidgetRuntime(options: WidgetRuntimeOptions): WidgetRuntime {
  const credentials = options.credentials ?? CredentialProvider.fromWindow();
  const token = credentials.token;
  const client = new GatewayClient({
    host: credentials.host,
    port: credentials.port,
    token: () => credentials.token,
    topics: options.topics ?? DEFAULT_TOPICS,
    heartbeatMs: options.heartbeatMs,
  });

  const runtime: WidgetRuntime = {
    client,
    credentials,
    hasToken: token !== null,
    status: client.currentStatus,
    start: () => {
      credentials.rememberFragmentToken();
      if (!credentials.token) {
        log.warn('starting without a gateway token; alerts resume once credentials exist');
        return;
      }
      client.connect();
    },
    stop: () => client.disconnect(),
    injectTestEnvelope: (envelope) => {
      try {
        options.onEnvelope(envelope);
      } catch (error) {
        log.warn('test envelope handler threw', error);
      }
    },
  };

  client.onEvent((envelope) => {
    try {
      options.onEnvelope(envelope);
    } catch (error) {
      log.warn('envelope handler threw; continuing with next frame', error);
    }
  });
  client.onGap((gap) => {
    // Alert widgets never replay after a gap: log and keep new events only.
    log.warn(`event gap observed (lost ${gap.lost}); skipping resync by design`);
    options.onGap?.(gap);
  });
  client.onStatus((status) => {
    runtime.status = status;
    options.onStatus?.(status);
  });
  return runtime;
}
