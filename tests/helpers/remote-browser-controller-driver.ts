import { chromium } from '@playwright/test';
import { createInterface } from 'node:readline';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';

const lines = createInterface({ input: process.stdin });
const iterator = lines[Symbol.asyncIterator]();
const first = await iterator.next();
if (first.done) process.exit(2);
const input = JSON.parse(first.value) as { origin: string; inviteId: string; secret: string; replacementId: string; replacementSecret: string; signals: string };
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ ignoreHTTPSErrors: true });
const page = await context.newPage();
let leaked = false;
context.on('request', (request) => { if (request.url().includes(input.secret) || request.url().includes(input.replacementSecret)) leaked = true; });
const mark = (name: string) => writeFileSync(join(input.signals, name), 'ok');
try {
  const fragment = new URLSearchParams({ inviteId: input.inviteId, secret: input.secret });
  await page.goto(`${input.origin}/remote/v1/join#${fragment}`, { waitUntil: 'domcontentloaded' });
  await page.waitForFunction(() => location.hash === '');
  await page.locator('#label').fill('Browser moderator');
  await page.locator('#join button').click();
  mark('requested');
  await page.locator('#open-app').waitFor({ state: 'visible', timeout: 20_000 });
  await page.locator('#open-app').click();
  await page.getByRole('button', { name: /Host initial/ }).waitFor({ timeout: 20_000 });
  await page.getByRole('button', { name: /Review command storage|Revisar almacenamiento de comandos/ }).click();
  await page.getByRole('button', { name: /I reviewed the host; create journal|Revisé el estado; crear registro/ }).click();
  await page.getByRole('button', { name: /Review command storage|Revisar almacenamiento de comandos/ }).waitFor({ state: 'hidden', timeout: 20_000 });
  const cookies = await context.cookies();
  if (!cookies.some((cookie) => cookie.name === '__Host-tiktools-remote' && cookie.httpOnly && cookie.secure)) throw new Error('session cookie flags missing');
  if (await page.evaluate(() => Object.keys(localStorage).some((key) => /token|credential|secret/i.test(key)))) throw new Error('credential-like browser storage');
  if (leaked) throw new Error('invitation secret appeared in a request');
  mark('ready');
  await page.getByRole('button', { name: /Host updated/ }).waitFor({ timeout: 20_000 });
  mark('updated');
  const built = await Bun.build({ entrypoints: ['tests/helpers/browser-command-test-entry.ts'], target: 'browser', format: 'esm' });
  if (!built.success || built.outputs.length !== 1) throw new Error('command fixture build failed');
  const bundle = await built.outputs[0]!.text();
  await context.route('**/__command-fixture.js', (route) => route.fulfill({ contentType: 'text/javascript', body: bundle }));
  const commandId = await page.evaluate(async () => {
    const path = '/__command-fixture.js';
    const api = await import(/* @vite-ignore */ path);
    const transport = api.createRemoteBrowserTransport();
    const remote = api.createControlClient({ transport });
    remote.attach();
    try {
      const hello = await transport.connect();
      const binding = { origin: location.origin, hostId: hello.hostId, principalId: hello.principalId, sessionId: hello.sessionId };
      const getRevision = async (id: string) => {
        let versions: Array<{ id: string; revision: string }> = [];
        await remote.call('workflows.get', { id }, { onMetadata: (meta: { resourceVersions: typeof versions }) => { versions = meta.resourceVersions; } });
        const match = versions.find((version) => version.id === id);
        if (!match) throw new Error('workflow version missing');
        return match.revision;
      };
      await api.sendBrowserWorkflowCommand({ remote, transport, expected: hello, binding,
        target: { operation: 'disable', resourceId: 'welcome', expectedRevision: await getRevision('welcome') }, isCurrent: () => true });
      const lossy = { ...remote, call: async (...args: unknown[]) => {
        const result = await remote.call(...args);
        if (args[0] === 'workflows.delete') throw new Error('fixture discarded the reply');
        return result;
      } };
      let lost = false;
      try { await api.sendBrowserWorkflowCommand({ remote: lossy, transport, expected: hello, binding,
        target: { operation: 'delete', resourceId: 'other', expectedRevision: await getRevision('other') }, isCurrent: () => true }); }
      catch { lost = true; }
      if (!lost) throw new Error('fixture did not discard reply');
      const pending = await api.withBrowserCommandJournal(binding, (record: { pending: { commandId: string } | null }) => structuredClone(record.pending));
      if (!pending) throw new Error('missing pending command');
      return pending.commandId as string;
    } finally { remote.detach(); }
  });
  await page.reload();
  await page.getByRole('button', { name: /Host updated/ }).waitFor({ timeout: 20_000 });
  await page.getByRole('button', { name: /Check outcome|Consultar resultado/ }).click({ timeout: 20_000 });
  await page.getByRole('button', { name: /Check outcome|Consultar resultado/ }).waitFor({ state: 'hidden', timeout: 20_000 });
  await page.evaluate(async (commandId) => {
    const path = '/__command-fixture.js';
    const api = await import(/* @vite-ignore */ path);
    const transport = api.createRemoteBrowserTransport(); const remote = api.createControlClient({ transport });
    remote.attach();
    try {
      const hello = await transport.connect();
      const binding = { origin: location.origin, hostId: hello.hostId, principalId: hello.principalId, sessionId: hello.sessionId };
      const saved = await api.withBrowserCommandJournal(binding, (record: { pending: unknown; recent: Array<{ outcome: string; intent: { commandId: string } }> }) => structuredClone(record));
      if (saved.pending || saved.recent.length !== 2 || saved.recent[0].intent.commandId !== commandId || saved.recent.some((entry: { outcome: string }) => entry.outcome !== 'succeeded')) throw new Error('invalid command journal');
    } finally { remote.detach(); }
  }, commandId);
  await page.getByRole('button', { name: /UI workflow/ }).click();
  await page.getByRole('button', { name: /Disable workflow|Desactivar workflow/ }).click();
  await page.getByRole('dialog').getByRole('button', { name: /Disable workflow|Desactivar workflow/ }).click();
  await page.getByRole('button', { name: /Disable workflow|Desactivar workflow/ }).waitFor({ state: 'hidden', timeout: 20_000 });
  mark('commands-complete');
  const replacement = await context.newPage();
  const secondFragment = new URLSearchParams({ inviteId: input.replacementId, secret: input.replacementSecret });
  await replacement.goto(`${input.origin}/remote/v1/join#${secondFragment}`, { waitUntil: 'domcontentloaded' });
  await replacement.waitForFunction(() => location.hash === '');
  await replacement.locator('#label').fill('Browser replacement');
  await replacement.locator('#join button').click();
  mark('replacement-requested');
  await replacement.locator('#open-app').waitFor({ state: 'visible', timeout: 20_000 });
  await page.getByRole('button', { name: /Host updated/ }).waitFor({ state: 'hidden', timeout: 20_000 });
  mark('cookie-cleared');
  await replacement.locator('#open-app').click();
  const liveStatus = replacement.getByRole('region', { name: /Host LIVE|LIVE del anfitrión/ });
  await liveStatus.getByText(/Not connected to LIVE|Sin conexión al LIVE/).waitFor({ timeout: 20_000 });
  await replacement.getByText(/Long message/).waitFor({ timeout: 20_000 });
  await replacement.getByText('Browser feed message').waitFor({ timeout: 20_000 });
  if (await replacement.getByRole('button', { name: /Host updated/ }).count()) throw new Error('LIVE-only observer received workflow');
  if (leaked) throw new Error('invitation secret appeared in a request');
  mark('replacement-ready');
  await liveStatus.waitFor({ state: 'hidden', timeout: 20_000 });
  await replacement.getByText('Browser feed message').waitFor({ state: 'hidden', timeout: 20_000 });
  mark('cleared');
} catch (error) {
  const description = error instanceof Error ? error.message : 'browser fixture failed';
  writeFileSync(join(input.signals, 'failure'), description.replaceAll(input.secret, '[redacted]').replaceAll(input.replacementSecret, '[redacted]'));
  process.exitCode = 1;
} finally {
  await context.close();
  await browser.close();
  lines.close();
}
