import { callWithMeta, type ControlClient } from '@/shared/api/control-client.ts';
import type { RemoteBrowserHello, RemoteBrowserTransport } from '@/shared/api/remote-browser-transport.ts';
import { beginBrowserCommand, finishBrowserCommand, withBrowserCommandJournal, type BrowserCommandBinding, type BrowserCommandIntent } from './browser-command-journal.ts';
import { verifyBrowserWorkflowOutcome } from './browser-command-recovery.ts';

export type BrowserWorkflowTarget = { operation: 'disable' | 'delete'; resourceId: string; expectedRevision: string };

/** Prepare, persist, recheck and send one confirmed intent. Transport errors and
 * unverifiable replies leave the original ID pending; this never retries. The
 * calling view must fence the exact selected row and its revision. This helper
 * is not yet connected to UI actions or a storage-loss recovery policy. */
export async function sendBrowserWorkflowCommand(options: {
  remote: ControlClient; transport: RemoteBrowserTransport; expected: RemoteBrowserHello;
  binding: BrowserCommandBinding; target: BrowserWorkflowTarget; isCurrent: () => boolean;
}): Promise<void> {
  const { remote, transport, expected, isCurrent } = options;
  const binding = { ...options.binding }; const target = { ...options.target };
  const identity = (): boolean => transport.currentHello() === expected && binding.origin === globalThis.location.origin &&
    binding.hostId === expected.hostId && binding.principalId === expected.principalId && binding.sessionId === expected.sessionId;
  const allowed = (): boolean => identity() && isCurrent() && expected.capabilities.includes('workflows.read') &&
    expected.capabilities.includes('commands.read') && expected.capabilities.includes(target.operation === 'disable' ? 'workflows.toggle' : 'workflows.edit') &&
    expected.features[target.operation === 'disable' ? 'workflowDisable' : 'workflowDelete'] === true &&
    (expected.resourceRestrictions['workflow'] === undefined || expected.resourceRestrictions['workflow'].includes(target.resourceId));
  if (!allowed() || !await transport.verifySession() || !allowed()) throw new Error('workflow confirmation changed');
  const intent: BrowserCommandIntent = { commandId: crypto.randomUUID(), operation: target.operation, resourceId: target.resourceId,
    hostInstanceId: expected.hostInstanceId, expectedRevision: target.expectedRevision, createdAt: Date.now() };
  await withBrowserCommandJournal(binding, (journal) => {
    if (!allowed()) throw new Error('workflow confirmation changed');
    beginBrowserCommand(journal, intent);
  });
  let ready = false;
  try { ready = allowed() && await transport.verifySession() && allowed(); } catch { /* known unsent */ }
  if (!ready) {
    await withBrowserCommandJournal(binding, (journal) => {
      finishBrowserCommand(journal, intent.commandId, { outcome: 'rejected', revision: null, errorCode: 'stale_view', recordedAt: Date.now() });
    });
    throw new Error('workflow confirmation changed before dispatch');
  }
  const reply = await callWithMeta<unknown>(remote, `workflows.${intent.operation}`, { id: intent.resourceId }, {
    meta: { commandId: intent.commandId, hostInstanceId: intent.hostInstanceId, expectedRevision: intent.expectedRevision },
  });
  if (!identity() || !await transport.verifySession() || !identity() || reply.meta?.hostInstanceId !== intent.hostInstanceId) throw new Error('command outcome not confirmed');
  const revision = verifyBrowserWorkflowOutcome(intent, binding.principalId, { principalId: binding.principalId,
    method: `workflows.${intent.operation}`, hostInstanceId: reply.meta.hostInstanceId, command: reply.meta.command,
    resourceVersions: reply.meta.resourceVersions, result: reply.result });
  if (revision === null) throw new Error('command outcome not confirmed');
  await withBrowserCommandJournal(binding, (journal) => {
    if (!identity() || !journal.pending || JSON.stringify(journal.pending) !== JSON.stringify(intent)) throw new Error('pending command changed');
    finishBrowserCommand(journal, intent.commandId, { outcome: 'succeeded', revision, errorCode: null, recordedAt: Date.now() });
  });
}
