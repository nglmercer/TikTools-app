import type { ControlClient } from '@/shared/api/control-client.ts';

export interface AssetRecord {
  readonly id: string;
  readonly mime: string;
  readonly width: number;
  readonly height: number;
  readonly size: number;
  /** RFC 3339. Absent on a Cloud that predates asset listing. */
  readonly createdAt?: string;
  /** Owner-chosen name, 1 to 120 characters. */
  readonly label?: string;
}

export interface PreflightItem {
  readonly kind: string;
  readonly id: string;
  readonly version: string | null;
  readonly revision: number | null;
  readonly required: boolean;
  readonly status: string;
  readonly permissions: string;
}

export interface PackagePreflight {
  readonly items: readonly PreflightItem[];
  readonly canApply: boolean;
  readonly applyMode: string;
}

export class InvalidBundleResponse extends Error {
  constructor() {
    super('invalid bundle response');
    this.name = 'InvalidBundleResponse';
  }
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
function count(value: unknown): number {
  if (typeof value !== 'number' || !Number.isFinite(value) || value < 0) throw new InvalidBundleResponse();
  return value;
}

export function parseAsset(value: unknown): AssetRecord {
  const asset = record(value);
  const id = text(asset.id);
  if (!UUID.test(id)) throw new InvalidBundleResponse();
  return {
    id,
    mime: text(asset.mime),
    width: count(asset.width),
    height: count(asset.height),
    size: count(asset.size),
    ...(typeof asset.createdAt === 'string' ? { createdAt: asset.createdAt } : {}),
    ...(typeof asset.label === 'string' && asset.label.length > 0 ? { label: asset.label } : {}),
  };
}

export function parsePreflight(value: unknown): PackagePreflight {
  const report = record(value);
  if (!Array.isArray(report.items) || report.applyMode !== 'clientJournal' || typeof report.canApply !== 'boolean') {
    throw new InvalidBundleResponse();
  }
  return {
    canApply: report.canApply,
    applyMode: report.applyMode,
    items: report.items.map((entry) => {
      const item = record(entry);
      return {
        kind: text(item.kind),
        id: text(item.id),
        version: typeof item.version === 'string' ? item.version : null,
        revision: typeof item.revision === 'number' ? item.revision : null,
        required: item.required === true,
        status: text(item.status),
        permissions: text(item.permissions),
      };
    }),
  };
}

/** Encodes raw bytes as base64 in chunks, so large images never overflow the call stack. */
export function toBase64(bytes: Uint8Array): string {
  let binary = '';
  for (let offset = 0; offset < bytes.length; offset += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
  }
  return btoa(binary);
}

export async function uploadAsset(client: ControlClient, bytes: Uint8Array): Promise<AssetRecord> {
  return parseAsset(await client.call<unknown>('bundles.assets.upload', { dataBase64: toBase64(bytes) }));
}

export async function packagePreflight(client: ControlClient, id: string): Promise<PackagePreflight> {
  return parsePreflight(await client.call<unknown>('bundles.packages.preflight', { id }));
}
