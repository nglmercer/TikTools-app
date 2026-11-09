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
