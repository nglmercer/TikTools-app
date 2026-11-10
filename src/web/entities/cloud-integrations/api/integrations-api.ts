import type { ControlClient } from '@/shared/api/control-client.ts';

export interface IntegrationConnection {
  readonly id: string;
  readonly provider: string;
  readonly label: string;
  readonly status: string;
  readonly createdAt: string;
}

/** Returned once by connect. The secret exists only in memory until the owner dismisses it. */
export interface IntegrationConnectResult {
  readonly connectionId: string;
  readonly provider: string;
  readonly label: string;
  readonly secret: string;
  readonly deliveryPath: string;
}

export interface IntegrationEvent {
  readonly deliveryId: string;
  readonly receivedAt: string;
  readonly payload: unknown;
}

export interface IntegrationEventList {
  readonly items: readonly IntegrationEvent[];
  readonly truncated: boolean;
}

export class InvalidIntegrationResponse extends Error {
  constructor() {
    super('invalid integration response');
    this.name = 'InvalidIntegrationResponse';
  }
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const SECRET = /^[0-9a-f]{64}$/i;
const LABEL_MAX = 120;

type Json = Record<string, unknown>;

function record(value: unknown): Json | null {
  return value !== null && typeof value === 'object' && !Array.isArray(value) ? (value as Json) : null;
}

function text(value: unknown): string {
  if (typeof value !== 'string') throw new InvalidIntegrationResponse();
  return value;
}

function connection(value: unknown): IntegrationConnection {
  const item = record(value);
  if (!item) throw new InvalidIntegrationResponse();
  const id = text(item.id);
  if (!UUID.test(id)) throw new InvalidIntegrationResponse();
  return {
    id,
    provider: text(item.provider),
    label: text(item.label).slice(0, LABEL_MAX),
    status: text(item.status),
    createdAt: text(item.createdAt),
  };
}

export function parseConnections(value: unknown): readonly IntegrationConnection[] {
  const page = record(value);
  if (!page || !Array.isArray(page.items)) throw new InvalidIntegrationResponse();
  return page.items.map(connection);
}

export function parseConnectResult(value: unknown): IntegrationConnectResult {
  const result = record(value);
  if (!result) throw new InvalidIntegrationResponse();
  const connectionId = text(result.connectionId);
  const secret = text(result.secret);
  if (!UUID.test(connectionId) || !SECRET.test(secret)) throw new InvalidIntegrationResponse();
  return {
    connectionId,
    provider: text(result.provider),
    label: text(result.label),
    secret,
    deliveryPath: text(result.deliveryPath),
  };
}

export function parseEvents(value: unknown): IntegrationEventList {
  const page = record(value);
  if (!page || !Array.isArray(page.items) || typeof page.truncated !== 'boolean') {
    throw new InvalidIntegrationResponse();
  }
  return {
    truncated: page.truncated,
    items: page.items.map((entry) => {
      const event = record(entry);
      if (!event) throw new InvalidIntegrationResponse();
      return {
        deliveryId: text(event.deliveryId),
        receivedAt: text(event.receivedAt),
        payload: event.payload,
      };
    }),
  };
}

export async function listConnections(client: ControlClient): Promise<readonly IntegrationConnection[]> {
  return parseConnections(await client.call<unknown>('integrations.connections', {}));
}

export async function connectWebhook(client: ControlClient, label: string): Promise<IntegrationConnectResult> {
  return parseConnectResult(await client.call<unknown>('integrations.connect', { label }));
}

export async function readEvents(client: ControlClient, id: string): Promise<IntegrationEventList> {
  return parseEvents(await client.call<unknown>('integrations.events', { id }));
}

export async function disconnect(client: ControlClient, id: string): Promise<void> {
  await client.call<unknown>('integrations.disconnect', { id });
}
