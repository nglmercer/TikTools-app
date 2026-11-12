import type { ResponseMetadata } from './control-contracts.ts';

export type RpcResponseWire = {
  id?: number | string | null;
  result?: unknown;
  meta?: unknown;
  error?: { code?: unknown; message?: unknown; data?: unknown };
};

export function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

export function isResponseMetadata(value: unknown): value is ResponseMetadata {
  if (!isRecord(value) || !Array.isArray(value['resourceVersions'])) return false;
  if (value['hostInstanceId'] !== undefined && typeof value['hostInstanceId'] !== 'string') return false;
  const command = value['command'];
  if (command !== undefined) {
    if (!isRecord(command) || typeof command['commandId'] !== 'string' ||
        !['accepted', 'running', 'succeeded', 'failed', 'unknown'].includes(String(command['state']))) return false;
    if (command['committedAt'] !== undefined && (typeof command['committedAt'] !== 'number' || !Number.isSafeInteger(command['committedAt']))) return false;
    const resource = command['resource'];
    if (resource !== undefined && (!isRecord(resource) || typeof resource['kind'] !== 'string' || typeof resource['id'] !== 'string' || typeof resource['scope'] !== 'string')) return false;
  }
  return value['resourceVersions'].every((version: unknown) => isRecord(version) &&
    typeof version['kind'] === 'string' && typeof version['id'] === 'string' &&
    typeof version['scope'] === 'string' && typeof version['revision'] === 'string' &&
    /^(0|[1-9][0-9]*)$/.test(version['revision']));
}
