import type { ControlClient } from '@/shared/api/control-client.ts';
import { isRuntimeVariable, type RuntimeVariable, type RuntimeVariablesSnapshot } from '../lib/runtime-variable.ts';

function typedSnapshot(result: unknown): RuntimeVariablesSnapshot {
  if (!result || typeof result !== 'object' || Array.isArray(result)) throw new Error('Invalid local variable response');
  const value = result as Record<string, unknown>;
  if (typeof value.revision !== 'string' || !/^[a-f0-9]{64}$/.test(value.revision) || !value.variables || typeof value.variables !== 'object' || Array.isArray(value.variables) || !Object.values(value.variables).every(isRuntimeVariable)) {
    throw new Error('Invalid local variable response');
  }
  return { revision: value.revision, variables: value.variables as Record<string, RuntimeVariable> };
}

export async function listTypedGlobals(client: ControlClient): Promise<RuntimeVariablesSnapshot> {
  return typedSnapshot(await client.call('globals.typed.list', {}));
}

export async function replaceTypedGlobals(client: ControlClient, expectedRevision: string, variables: Record<string, RuntimeVariable>): Promise<RuntimeVariablesSnapshot> {
  return typedSnapshot(await client.call('globals.typed.replace', { expectedRevision, variables }));
}

function asGlobalsMap(result: unknown): Record<string, string> {
  if (!result || typeof result !== 'object' || Array.isArray(result)) return {};
  const raw = (result as { globals?: unknown }).globals;
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {};
  return Object.fromEntries(
    Object.entries(raw as Record<string, unknown>).filter((entry): entry is [string, string] => typeof entry[1] === 'string'),
  );
}

export async function listGlobals(client: ControlClient): Promise<Record<string, string>> {
  return asGlobalsMap(await client.call('globals.list', {}));
}

export async function setGlobal(client: ControlClient, key: string, value: string): Promise<void> {
  await client.call('globals.set', { key, value });
}

export async function deleteGlobal(client: ControlClient, key: string): Promise<void> {
  await client.call('globals.delete', { key });
}
