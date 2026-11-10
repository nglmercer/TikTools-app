import { expect, test } from 'vitest';
import type { BrowserCommandIntent } from './browser-command-journal.ts';
import { verifyBrowserWorkflowOutcome } from './browser-command-recovery.ts';

const intent: BrowserCommandIntent = { commandId: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', operation: 'disable', resourceId: 'welcome', hostInstanceId: 'instance', expectedRevision: '9007199254740993', createdAt: 1 };
const outcome = () => ({ principalId: 'principal', hostInstanceId: 'instance', method: 'workflows.disable',
  command: { commandId: intent.commandId, state: 'succeeded', committedAt: 1, resource: { kind: 'workflow', id: 'welcome', scope: 'host' } },
  resourceVersions: [{ kind: 'workflow', id: 'welcome', scope: 'host', revision: '9007199254740994' }], result: { id: 'welcome', enabled: false } });

test('accepts only an exact durable workflow result and retains exact large revisions', () => {
  expect(verifyBrowserWorkflowOutcome(intent, 'principal', outcome())).toBe('9007199254740994');
  const invalid = [
    (v: ReturnType<typeof outcome>) => { v.principalId = 'other'; },
    (v: ReturnType<typeof outcome>) => { v.hostInstanceId = 'other'; },
    (v: ReturnType<typeof outcome>) => { v.method = 'workflows.delete'; },
    (v: ReturnType<typeof outcome>) => { v.command.commandId = 'different'; },
    (v: ReturnType<typeof outcome>) => { v.command.resource.id = 'other'; },
    (v: ReturnType<typeof outcome>) => { v.command.committedAt = Number.MAX_SAFE_INTEGER + 1; },
    (v: ReturnType<typeof outcome>) => { v.resourceVersions[0]!.revision = '9007199254740993'; },
    (v: ReturnType<typeof outcome>) => { v.resourceVersions[0]!.scope = 'other'; },
    (v: ReturnType<typeof outcome>) => { v.result.enabled = true; },
  ];
  for (const change of invalid) { const value = outcome(); change(value); expect(() => verifyBrowserWorkflowOutcome(intent, 'principal', value)).toThrow(); }
});

test('keeps non-success outcomes pending and checks deletion revision semantics', () => {
  for (const state of ['accepted', 'running', 'failed', 'unknown']) {
    const value = outcome(); value.command.state = state;
    expect(verifyBrowserWorkflowOutcome(intent, 'principal', value)).toBeNull();
  }
  const deleted = { ...intent, operation: 'delete' as const };
  const result = { ...outcome(), method: 'workflows.delete', result: { id: 'welcome', deleted: false },
    resourceVersions: [{ kind: 'workflow', scope: 'host', id: 'welcome', revision: intent.expectedRevision }] };
  expect(verifyBrowserWorkflowOutcome(deleted, 'principal', result)).toBe(intent.expectedRevision);
  result.result.deleted = true;
  expect(() => verifyBrowserWorkflowOutcome(deleted, 'principal', result)).toThrow();
});
