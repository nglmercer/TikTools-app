import { expect, test } from '@playwright/test';

const binding = { origin: 'https://host.example', hostId: 'host', principalId: 'principal', sessionId: 'session' };
const modulePath = '/features/controller/model/browser-command-journal.ts';

test('browser journal serializes tabs and retains uncertain commands across reload', async ({ page, context }) => {
  await context.route('**/__journal-test', (route) => route.fulfill({ contentType: 'text/html', body: '<!doctype html><title>Journal fixture</title>' }));
  await page.goto('/__journal-test');
  const other = await context.newPage();
  await other.goto('/__journal-test');
  const make = (commandId: string) => ({ commandId, operation: 'disable', resourceId: 'workflow', hostInstanceId: 'instance', expectedRevision: '2', createdAt: 1 });
  const write = (target: typeof page, commandId: string) => target.evaluate(async ({ modulePath, binding, intent }) => {
    const journal = await import(/* @vite-ignore */ modulePath);
    try {
      await journal.withBrowserCommandJournal(binding, (record: unknown) => journal.beginBrowserCommand(record, intent));
      return 'committed';
    } catch { return 'blocked'; }
  }, { modulePath, binding, intent: make(commandId) });
  const results = await Promise.all([write(page, 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa'), write(other, 'bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbbb')]);
  expect(results.sort()).toEqual(['blocked', 'committed']);
  await page.reload();
  const read = () => page.evaluate(async ({ modulePath, binding }) => {
    const journal = await import(/* @vite-ignore */ modulePath);
    return journal.withBrowserCommandJournal(binding, (record: unknown) => structuredClone(record));
  }, { modulePath, binding });
  const saved = await read();
  expect(saved.pending.resourceId).toBe('workflow');
  expect(saved.recent).toEqual([]);
  const wrong = await page.evaluate(async ({ modulePath, binding }) => {
    const journal = await import(/* @vite-ignore */ modulePath);
    try { await journal.withBrowserCommandJournal(binding, (record: unknown) => journal.finishBrowserCommand(record, 'cccccccc-cccc-4ccc-8ccc-cccccccccccc', { outcome: 'reviewed-unknown', revision: null, errorCode: null, recordedAt: 2 })); return false; }
    catch { return true; }
  }, { modulePath, binding });
  expect(wrong).toBe(true);
  expect((await read()).pending.commandId).toBe(saved.pending.commandId);
  await page.evaluate(async ({ modulePath, binding, commandId }) => {
    const journal = await import(/* @vite-ignore */ modulePath);
    await journal.withBrowserCommandJournal(binding, (record: unknown) => journal.finishBrowserCommand(record, commandId, { outcome: 'reviewed-unknown', revision: null, errorCode: null, recordedAt: 2 }));
  }, { modulePath, binding, commandId: saved.pending.commandId });
  await other.reload();
  const after = await other.evaluate(async ({ modulePath, binding }) => {
    const journal = await import(/* @vite-ignore */ modulePath);
    return journal.withBrowserCommandJournal(binding, (record: unknown) => structuredClone(record));
  }, { modulePath, binding });
  expect(after.pending).toBeNull();
  expect(after.recent[0].outcome).toBe('reviewed-unknown');
});

test('failed transactions roll back and malformed stored data fails closed', async ({ page, context }) => {
  await context.route('**/__journal-test', (route) => route.fulfill({ contentType: 'text/html', body: '<!doctype html><title>Journal fixture</title>' }));
  await page.goto('/__journal-test');
  const output = await page.evaluate(async ({ modulePath, binding }) => {
    const journal = await import(/* @vite-ignore */ modulePath);
    const intent = { commandId: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', operation: 'disable', resourceId: 'workflow', hostInstanceId: 'instance', expectedRevision: '2', createdAt: 1 };
    let aborted = false;
    try {
      await journal.withBrowserCommandJournal(binding, (record: unknown) => { journal.beginBrowserCommand(record, intent); throw new Error('abort'); });
    } catch { aborted = true; }
    const state = await journal.withBrowserCommandJournal(binding, (record: unknown) => structuredClone(record));
    let asyncRejected = false;
    try { await journal.withBrowserCommandJournal(binding, async () => true); } catch { asyncRejected = true; }
    await new Promise<void>((resolve, reject) => {
      const open = indexedDB.open('tiktools-remote-command-journal-v1', 1);
      open.onsuccess = () => {
        const db = open.result; const tx = db.transaction('journals', 'readwrite');
        tx.objectStore('journals').put({ key: JSON.stringify([binding.origin, binding.hostId, binding.principalId, binding.sessionId]), version: 99 });
        tx.oncomplete = () => { db.close(); resolve(); }; tx.onabort = () => reject(new Error('fixture failed'));
      };
      open.onerror = () => reject(new Error('fixture failed'));
    });
    let corrupted = false;
    try { await journal.withBrowserCommandJournal(binding, () => null); } catch { corrupted = true; }
    return { aborted, pending: state.pending, asyncRejected, corrupted };
  }, { modulePath, binding });
  expect(output).toEqual({ aborted: true, pending: null, asyncRejected: true, corrupted: true });
});

test('recovery queries the original command without replay and persists only verified success', async ({ page, context, baseURL }) => {
  await context.route('https://host.example/**', async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === '/__journal-test') await route.fulfill({ contentType: 'text/html', body: '<!doctype html><title>Journal fixture</title>' });
    else await route.fulfill({ response: await context.request.get(new URL(path, baseURL).href) });
  });
  await page.goto('https://host.example/__journal-test');
  const result = await page.evaluate(async ({ modulePath }) => {
    const journal = await import(/* @vite-ignore */ modulePath);
    const recoveryPath = '/features/controller/model/browser-command-recovery.ts';
    const recovery = await import(/* @vite-ignore */ recoveryPath);
    const binding = { origin: location.origin, hostId: 'host', principalId: 'principal', sessionId: 'session' };
    const intent = { commandId: 'aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa', operation: 'disable', resourceId: 'workflow', hostInstanceId: 'instance', expectedRevision: '2', createdAt: 1 };
    const hello = { ...binding, capabilities: ['commands.read', 'workflows.read'], resourceRestrictions: { workflow: ['workflow'] } };
    let active = true; let switchAfterRead = false;
    const transport = { currentHello: () => active ? hello : undefined, verifySession: async () => active };
    await journal.withBrowserCommandJournal(binding, (record: unknown) => journal.beginBrowserCommand(record, intent));
    const calls: string[] = [];
    let state = 'unknown';
    const remote = { call: async (method: string) => {
      calls.push(method);
      if (switchAfterRead) active = false;
      return { principalId: 'principal', hostInstanceId: 'instance', method: 'workflows.disable',
        command: { commandId: intent.commandId, state, committedAt: 1, resource: { kind: 'workflow', id: 'workflow', scope: 'host' } },
        resourceVersions: [{ kind: 'workflow', id: 'workflow', scope: 'host', revision: '3' }], result: { id: 'workflow', enabled: false } };
    } };
    const pending = await recovery.recoverBrowserWorkflowCommand(remote, transport, hello, binding, intent.commandId);
    const before = await journal.withBrowserCommandJournal(binding, (record: { pending: unknown }) => structuredClone(record.pending));
    state = 'succeeded';
    switchAfterRead = true;
    let switched = false;
    try { await recovery.recoverBrowserWorkflowCommand(remote, transport, hello, binding, intent.commandId); } catch { switched = true; }
    const afterSwitch = await journal.withBrowserCommandJournal(binding, (record: { pending: unknown }) => structuredClone(record.pending));
    active = true; switchAfterRead = false;
    const resolved = await recovery.recoverBrowserWorkflowCommand(remote, transport, hello, binding, intent.commandId);
    const after = await journal.withBrowserCommandJournal(binding, (record: unknown) => structuredClone(record));
    return { pending, before, switched, afterSwitch, resolved, after, calls };
  }, { modulePath });
  expect(result.pending).toBe('pending');
  expect(result.before.commandId).toBe('aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaaa');
  expect(result.switched).toBe(true);
  expect(result.afterSwitch.commandId).toBe(result.before.commandId);
  expect(result.resolved).toBe('resolved');
  expect(result.after.pending).toBeNull();
  expect(result.after.recent[0].revision).toBe('3');
  expect(result.calls).toEqual(['collaboration.commands.get', 'collaboration.commands.get', 'collaboration.commands.get']);
});

test('workflow dispatch commits intent before sending and retains unknown outcomes', async ({ page, context, baseURL }) => {
  await context.route('https://host.example/**', async (route) => {
    const path = new URL(route.request().url()).pathname;
    if (path === '/__journal-test') await route.fulfill({ contentType: 'text/html', body: '<!doctype html><title>Journal fixture</title>' });
    else await route.fulfill({ response: await context.request.get(new URL(path, baseURL).href) });
  });
  await page.goto('https://host.example/__journal-test');
  const result = await page.evaluate(async ({ modulePath }) => {
    const journal = await import(/* @vite-ignore */ modulePath);
    const commandPath = '/features/controller/model/browser-workflow-command.ts';
    const command = await import(/* @vite-ignore */ commandPath);
    async function scenario(sessionId: string, mode: 'success' | 'lost' | 'stale' | 'forged') {
      const binding = { origin: location.origin, hostId: 'host', principalId: 'principal', sessionId };
      const hello = { ...binding, hostInstanceId: 'instance', capabilities: ['commands.read', 'workflows.read', 'workflows.toggle'],
        resourceRestrictions: { workflow: ['workflow'] }, features: { workflowDisable: true } };
      let checks = 0; const calls: string[] = []; let intentWasPersisted = false;
      const transport = { currentHello: () => hello, verifySession: async () => ++checks !== (mode === 'stale' ? 2 : -1) };
      const remote = { call: async (method: string, params: { id: string }, options: { meta: { commandId: string }; onMetadata: (v: unknown) => void }) => {
        calls.push(method);
        const pending = await journal.withBrowserCommandJournal(binding, (record: { pending: { commandId: string } | null }) => structuredClone(record.pending));
        intentWasPersisted = pending?.commandId === options.meta.commandId;
        if (mode === 'lost') throw new Error('connection lost');
        options.onMetadata({ hostInstanceId: 'instance', command: { commandId: options.meta.commandId, state: 'succeeded', committedAt: 1,
          resource: { kind: 'workflow', id: params.id, scope: 'host' } }, resourceVersions: [{ kind: 'workflow', id: params.id, scope: 'host', revision: mode === 'forged' ? '7' : '3' }] });
        return { id: params.id, enabled: false };
      } };
      let failed = false;
      try { await command.sendBrowserWorkflowCommand({ remote, transport, expected: hello, binding,
        target: { operation: 'disable', resourceId: 'workflow', expectedRevision: '2' }, isCurrent: () => true }); }
      catch { failed = true; }
      const stored = await journal.withBrowserCommandJournal(binding, (record: unknown) => structuredClone(record));
      return { calls, failed, intentWasPersisted, stored };
    }
    return { success: await scenario('success', 'success'), lost: await scenario('lost', 'lost'), stale: await scenario('stale', 'stale'), forged: await scenario('forged', 'forged') };
  }, { modulePath });
  expect(result.success.calls).toEqual(['workflows.disable']);
  expect(result.success.intentWasPersisted).toBe(true);
  expect(result.success.stored.pending).toBeNull();
  expect(result.success.stored.recent[0].outcome).toBe('succeeded');
  expect(result.lost.calls).toEqual(['workflows.disable']);
  expect(result.lost.failed).toBe(true);
  expect(result.lost.intentWasPersisted).toBe(true);
  expect(result.lost.stored.pending.operation).toBe('disable');
  expect(result.stale.calls).toEqual([]);
  expect(result.stale.stored.pending).toBeNull();
  expect(result.stale.stored.recent[0].outcome).toBe('rejected');
  expect(result.forged.failed).toBe(true);
  expect(result.forged.stored.pending.operation).toBe('disable');
});
