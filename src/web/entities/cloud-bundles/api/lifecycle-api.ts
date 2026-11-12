import type { ControlClient } from '@/shared/api/control-client.ts';
import { InvalidBundleResponse, parseAsset, type AssetRecord } from './bundles-api.ts';

/** Largest page the Cloud serves for assets and overlays. */
export const CLOUD_PAGE_SIZE = 50;

export interface Page<T> {
  readonly items: readonly T[];
  readonly nextCursor: string | null;
}

export interface OverlaySummary {
  readonly id: string;
  readonly kind: string;
  /** The document title (the overlay schema has no separate name). */
  readonly name: string;
  readonly revision: number;
  readonly status: string;
  readonly createdAt: string;
  readonly updatedAt: string;
  readonly elementCount: number;
}

export interface OverlayRevisionSummary {
  readonly revision: number;
  readonly status: string;
  readonly name: string;
  readonly sha256: string;
  readonly createdAt: string;
  readonly current: boolean;
}

export interface OverlayRevisionPage extends Page<OverlayRevisionSummary> {
  readonly id: string;
  readonly currentRevision: number;
}

export interface OverlayRevisionRecord {
  readonly id: string;
  readonly revision: number;
  readonly status: string;
  readonly schemaVersion: number;
  readonly sha256: string;
  readonly createdAt: string;
  readonly current: boolean;
  readonly document: unknown;
}

type Json = Record<string, unknown>;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function record(value: unknown): Json {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) throw new InvalidBundleResponse();
  return value as Json;
}
function text(value: unknown): string {
  if (typeof value !== 'string') throw new InvalidBundleResponse();
  return value;
}
function whole(value: unknown): number {
  if (typeof value !== 'number' || !Number.isInteger(value) || value < 0) throw new InvalidBundleResponse();
  return value;
}
function uuid(value: unknown): string {
  const id = text(value);
  if (!UUID.test(id)) throw new InvalidBundleResponse();
  return id;
}
function cursor(value: unknown): string | null {
  if (value === null || value === undefined) return null;
  return text(value);
}
function items(value: unknown): unknown[] {
  if (!Array.isArray(value)) throw new InvalidBundleResponse();
  return value;
}

export function parseAssetPage(value: unknown): Page<AssetRecord> {
  const page = record(value);
  return { items: items(page.items).map(parseAsset), nextCursor: cursor(page.nextCursor) };
}

function parseOverlaySummary(value: unknown): OverlaySummary {
  const overlay = record(value);
  return {
    id: uuid(overlay.id),
    kind: text(overlay.kind),
    name: text(overlay.name),
    revision: whole(overlay.revision),
    status: text(overlay.status),
    createdAt: text(overlay.createdAt),
    updatedAt: text(overlay.updatedAt),
    elementCount: whole(overlay.elementCount),
  };
}

export function parseOverlayPage(value: unknown): Page<OverlaySummary> {
  const page = record(value);
  return { items: items(page.items).map(parseOverlaySummary), nextCursor: cursor(page.nextCursor) };
}

function parseRevisionSummary(value: unknown): OverlayRevisionSummary {
  const revision = record(value);
  return {
    revision: whole(revision.revision),
    status: text(revision.status),
    name: text(revision.name),
    sha256: text(revision.sha256),
    createdAt: text(revision.createdAt),
    current: revision.current === true,
  };
}

export function parseRevisionPage(value: unknown): OverlayRevisionPage {
  const page = record(value);
  return {
    id: uuid(page.id),
    currentRevision: whole(page.currentRevision),
    items: items(page.items).map(parseRevisionSummary),
    nextCursor: cursor(page.nextCursor),
  };
}

export function parseRevisionRecord(value: unknown): OverlayRevisionRecord {
  const revision = record(value);
  return {
    id: uuid(revision.id),
    revision: whole(revision.revision),
    status: text(revision.status),
    schemaVersion: whole(revision.schemaVersion),
    sha256: text(revision.sha256),
    createdAt: text(revision.createdAt),
    current: revision.current === true,
    document: record(revision.document),
  };
}

function parseDeleted(value: unknown, id: string): void {
  const reply = record(value);
  if (reply.id !== id || reply.deleted !== true) throw new InvalidBundleResponse();
}

const paging = (after: string | null, limit: number): Record<string, unknown> => ({
  limit,
  ...(after ? { cursor: after } : {}),
});

export async function listAssets(client: ControlClient, after: string | null, limit = CLOUD_PAGE_SIZE): Promise<Page<AssetRecord>> {
  return parseAssetPage(await client.call<unknown>('bundles.assets.list', paging(after, limit)));
}

/** `null` clears the label. */
export async function setAssetLabel(client: ControlClient, id: string, label: string | null): Promise<AssetRecord> {
  return parseAsset(await client.call<unknown>('bundles.assets.setLabel', { id, label }));
}

export async function deleteAsset(client: ControlClient, id: string): Promise<void> {
  parseDeleted(await client.call<unknown>('bundles.assets.delete', { id }), id);
}

export async function listOverlays(client: ControlClient, after: string | null, limit = CLOUD_PAGE_SIZE): Promise<Page<OverlaySummary>> {
  return parseOverlayPage(await client.call<unknown>('bundles.overlays.list', paging(after, limit)));
}

export async function listRevisions(client: ControlClient, id: string, after: string | null, limit = CLOUD_PAGE_SIZE): Promise<OverlayRevisionPage> {
  return parseRevisionPage(await client.call<unknown>('bundles.overlays.revisions', { id, ...paging(after, limit) }));
}

export async function readRevision(client: ControlClient, id: string, revision: number): Promise<OverlayRevisionRecord> {
  return parseRevisionRecord(await client.call<unknown>('bundles.overlays.revision', { id, revision }));
}

export async function deleteOverlay(client: ControlClient, id: string): Promise<void> {
  parseDeleted(await client.call<unknown>('bundles.overlays.delete', { id }), id);
}

export async function deletePackage(client: ControlClient, id: string): Promise<void> {
  parseDeleted(await client.call<unknown>('bundles.packages.delete', { id }), id);
}
