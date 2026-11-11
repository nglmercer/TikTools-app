import type { ControlClient } from '@/shared/api/control-client.ts';
import type { OverlayDocument } from '../../../../shared/cloud-overlay-schema.ts';

export interface OverlayRecord {
  readonly id: string;
  readonly revision: number;
  readonly status: string;
  readonly document: OverlayDocument;
}

export interface OverlayWrite {
  readonly id: string;
  readonly revision: number;
  readonly status: string;
}

export class InvalidOverlayResponse extends Error {
  constructor() {
    super('invalid overlay response');
    this.name = 'InvalidOverlayResponse';
  }
}

type Json = Record<string, unknown>;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function record(value: unknown): Json {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) throw new InvalidOverlayResponse();
  return value as Json;
}

function write(value: unknown): OverlayWrite {
  const reply = record(value);
  if (typeof reply.id !== 'string' || !UUID.test(reply.id) || typeof reply.revision !== 'number' || typeof reply.status !== 'string') {
    throw new InvalidOverlayResponse();
  }
  return { id: reply.id, revision: reply.revision, status: reply.status };
}

export function parseOverlayRecord(value: unknown): OverlayRecord {
  const reply = record(value);
  if (typeof reply.id !== 'string' || !UUID.test(reply.id) || typeof reply.revision !== 'number' || typeof reply.status !== 'string') {
    throw new InvalidOverlayResponse();
  }
  // The document is the Cloud's own copy; the editor validates it before use.
  return { id: reply.id, revision: reply.revision, status: reply.status, document: record(reply.document) as unknown as OverlayDocument };
}

export async function createOverlay(client: ControlClient, document: OverlayDocument): Promise<OverlayWrite> {
  return write(await client.call<unknown>('bundles.overlays.create', { document }));
}

export async function readOverlay(client: ControlClient, id: string): Promise<OverlayRecord> {
  return parseOverlayRecord(await client.call<unknown>('bundles.overlays.read', { id }));
}

export async function reviseOverlay(
  client: ControlClient,
  id: string,
  document: OverlayDocument,
  expectedRevision: number,
): Promise<OverlayWrite> {
  return write(await client.call<unknown>('bundles.overlays.revise', { id, document, expectedRevision }));
}
