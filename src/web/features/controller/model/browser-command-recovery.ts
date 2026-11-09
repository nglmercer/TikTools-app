import type { ControlClient } from '@/shared/api/control-client.ts';
import type { RemoteBrowserHello, RemoteBrowserTransport } from '@/shared/api/remote-browser-transport.ts';
import { finishBrowserCommand, withBrowserCommandJournal, withBrowserCommandLock, type BrowserCommandBinding, type BrowserCommandIntent } from './browser-command-journal.ts';

const object = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);
const decimal = (v: unknown): v is string => typeof v === 'string' && /^(0|[1-9][0-9]{0,18})$/.test(v) && BigInt(v) <= 9223372036854775807n;
const protocol = (): Error => new Error('invalid command outcome');

/** A retained outcome proves success only for the exact original intent. */
export function verifyBrowserWorkflowOutcome(intent: BrowserCommandIntent, principalId: string, value: unknown): string | null {
  if (!object(value) || value['principalId'] !== principalId || value['hostInstanceId'] !== intent.hostInstanceId ||
      value['method'] !== `workflows.${intent.operation}` || !object(value['command']) ||
      value['command']['commandId'] !== intent.commandId) throw protocol();
  const command = value['command'];
  if (command['state'] === 'accepted' || command['state'] === 'running' || command['state'] === 'failed' || command['state'] === 'unknown') return null;
  if (command['state'] !== 'succeeded' || typeof command['committedAt'] !== 'number' || !Number.isSafeInteger(command['committedAt']) || command['committedAt'] < 0 ||
      !object(command['resource']) || command['resource']['kind'] !== 'workflow' || command['resource']['scope'] !== 'host' || command['resource']['id'] !== intent.resourceId ||
      !Array.isArray(value['resourceVersions']) || value['resourceVersions'].length !== 1 || !object(value['resourceVersions'][0]) ||
      !object(value['result']) || value['result']['id'] !== intent.resourceId || !decimal(intent.expectedRevision)) throw protocol();
  const version = value['resourceVersions'][0];
  if (Object.keys(version).length !== 4 || version['kind'] !== 'workflow' || version['scope'] !== 'host' || version['id'] !== intent.resourceId || !decimal(version['revision'])) throw protocol();
  const result = value['result'];
  let changed: boolean;
  if (intent.operation === 'disable') {
    if (result['enabled'] !== false) throw protocol();
    changed = true;
  } else {
    if (typeof result['deleted'] !== 'boolean') throw protocol();
    changed = result['deleted'];
  }
  if (BigInt(version['revision']) !== BigInt(intent.expectedRevision) + (changed ? 1n : 0n)) throw protocol();
  return version['revision'];
}

/** Resolves one pending ID without replaying its mutation. A failed/unknown or
 * inaccessible outcome remains pending. */
export async function recoverBrowserWorkflowCommand(
  remote: ControlClient, transport: RemoteBrowserTransport, expected: RemoteBrowserHello, binding: BrowserCommandBinding, commandId: string,
): Promise<'resolved' | 'pending'> {
  binding = { ...binding };
  return withBrowserCommandLock(binding, async () => {
  const current = (): boolean => transport.currentHello() === expected && binding.origin === globalThis.location.origin && binding.hostId === expected.hostId &&
    binding.principalId === expected.principalId && binding.sessionId === expected.sessionId;
  if (!current() || !expected.capabilities.includes('commands.read') || !await transport.verifySession() || !current()) throw new Error('browser session changed');
  const intent = await withBrowserCommandJournal(binding, (journal) => {
    if (!journal.pending || journal.pending.commandId !== commandId) throw new Error('pending command changed');
    return structuredClone(journal.pending);
  });
  if (!expected.capabilities.includes('workflows.read') ||
      (expected.resourceRestrictions['workflow'] !== undefined && !expected.resourceRestrictions['workflow'].includes(intent.resourceId))) throw new Error('workflow is not permitted');
  if (!current() || !await transport.verifySession() || !current()) throw new Error('browser session changed');
  const result = await remote.call<unknown>('collaboration.commands.get', { commandId });
  const revision = verifyBrowserWorkflowOutcome(intent, binding.principalId, result);
  if (!current() || !await transport.verifySession() || !current()) throw new Error('browser session changed');
  if (revision === null) return 'pending';
  await withBrowserCommandJournal(binding, (journal) => {
    if (!current() || !journal.pending || JSON.stringify(journal.pending) !== JSON.stringify(intent)) throw new Error('pending command changed');
    finishBrowserCommand(journal, commandId, { outcome: 'succeeded', revision, errorCode: null, recordedAt: Date.now() });
  });
  return 'resolved';
  });
}
