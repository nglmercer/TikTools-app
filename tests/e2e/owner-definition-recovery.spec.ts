/** Real packaged Vue → local adapter → IPC → Core. One real return frame is
 * deliberately discarded after commit; no backend result is fabricated. */
import { expect, test, type BrowserContext, type Locator, type Page, type WebSocketRoute } from '@playwright/test';
import { execFile } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import { promisify } from 'node:util';
import english from '../../src/web/shared/i18n/locales/en.ts';
import spanish from '../../src/web/shared/i18n/locales/es.ts';

const execute = promisify(execFile);
const ownerUrl = process.env['TIKTOOLS_OWNER_DEFINITION_URL'];
const home = process.env['TIKTOOLS_OWNER_DEFINITION_HOME'];
const binary = process.env['TIKTOOLS_OWNER_DEFINITION_BINARY'];
const shots = '/home/meme/.cache/tiktools-test-runs';
type Json = Record<string, unknown>;
interface Sent { id: string | number; method: string; params: Json; meta?: unknown }
interface Intent { commandId: string; operation: string; kind: string; id: string; expectedRevision: string; hostInstanceId: string }
interface NativeRecord { intent: Intent; outcome: string; receipt: { revision: string; affectedResources: number } | null }
interface NativeState { hostId: string; hostInstanceId: string; pending: Intent | null; recent: NativeRecord[] }
let cookies: Awaited<ReturnType<BrowserContext['cookies']>> = [];

async function rpc<T = Json>(method: string, params: Json = {}): Promise<T> {
  if (!home || !binary) throw new Error('Real native definition fixture is unavailable');
  const env: Record<string, string | undefined> = { ...process.env, TIKTOOLS_HOME: home, TIKTOOLS_CLOUD_ORIGIN: '' };
  for (const [key, directory] of Object.entries({ TIKTOOLS_DATA_DIR: 'data', TIKTOOLS_PLUGINS_DIR: 'plugins',
    TIKTOOLS_PLUGIN_DATA_DIR: 'plugin-data', TIKTOOLS_BUILTIN_PLUGINS_DIR: 'builtins', TIKTOOLS_DEV_PLUGINS_DIR: 'dev-plugins',
    TIKTOOLS_LOG_DIR: 'logs', TIKTOOLS_TEMP_DIR: 'temp' })) env[key] = `${home}/${directory}`;
  const output = await execute(binary, ['--json', 'api', 'call', method, JSON.stringify(params), '--confirm'], { cwd: home, env, maxBuffer: 8 * 1024 * 1024 });
  return JSON.parse(output.stdout) as T;
}
function target(request: Sent): string { return request.params['operation'] === 'create'
  ? (request.params['record'] as Json)['id'] as string : request.params['id'] as string; }
async function completed(request: Sent): Promise<NativeRecord> {
  let found: NativeRecord | undefined;
  await expect.poll(async () => {
    const state = await rpc<NativeState>('automation.definition.get');
    found = state.recent.find((entry) => entry.intent.id === target(request) && entry.intent.kind === request.params['kind'] &&
      entry.intent.operation === request.params['operation'] && entry.intent.expectedRevision === request.params['expectedRevision'] &&
      entry.intent.hostInstanceId === request.params['hostInstanceId']);
    return { pending: state.pending, outcome: found?.outcome ?? null };
  }).toEqual({ pending: null, outcome: 'succeeded' });
  return found!;
}
async function submitted(sent: Sent[], before: number): Promise<Sent> {
  await expect.poll(() => sent.length).toBe(before + 1);
  return sent[before]!;
}
async function settledEditor(page: Page, words: typeof english | typeof spanish): Promise<void> {
  await expect(page.locator('.automation-edit-notice').getByText(words.automations.conflict.waiting, { exact: true })).toHaveCount(0);
}
function editor(page: Page, kind: 'action' | 'event'): Locator { return page.locator(`.${kind}-editor:visible`); }
async function finishNavigation(page: Page, words: typeof english | typeof spanish, finished: () => Promise<boolean>): Promise<void> {
  const discard = page.getByRole('dialog', { name: words.automations.editor.discardTitle, exact: true });
  await expect.poll(async () => await discard.isVisible() || await finished()).toBe(true);
  if (await discard.isVisible()) {
    await discard.getByRole('button', { name: words.automations.editor.discardConfirm, exact: true }).click();
    await expect(discard).not.toBeVisible();
  }
  await expect.poll(finished).toBe(true);
}
async function backToLibrary(page: Page, words: typeof english | typeof spanish): Promise<void> {
  const sheet = page.locator('.automations-page__sheet');
  if (await sheet.isVisible()) {
    await sheet.getByRole('button', { name: words.common.back, exact: true }).click();
    await finishNavigation(page, words, async () => !await sheet.isVisible());
  }
}
async function select(page: Page, kind: 'action' | 'event', name: string, words: typeof english | typeof spanish): Promise<void> {
  await backToLibrary(page, words);
  const tab = page.getByRole('tab', { name: new RegExp(`^${kind === 'action' ? words.automations.library.actions : words.automations.library.events} \\(`) });
  await tab.click();
  await finishNavigation(page, words, async () => await tab.getAttribute('aria-selected') === 'true');
  const row = page.locator('.automation-library tbody tr').filter({ hasText: name });
  await expect(row).toBeVisible();
  // A restored selection opens its narrow sheet when the asynchronous snapshot
  // arrives. Wait for that snapshot before closing a sheet and choosing a row.
  await backToLibrary(page, words);
  await row.click();
  await expect(editor(page, kind).locator('input[type="text"]').first()).toHaveValue(name);
}
async function closeDialog(view: Locator, words: typeof english | typeof spanish): Promise<void> {
  await view.getByRole('button', { name: words.common.close, exact: true }).click();
  await expect(view).not.toBeVisible();
}
async function storageIsPrivate(page: Page, locale: string, commands: string[]): Promise<void> {
  const stored = await page.evaluate(() => JSON.stringify({ local: Object.entries(localStorage), session: Object.entries(sessionStorage) }));
  for (const value of ['owner-definition-', `definition-source-${locale}`, `Newer local draft ${locale}`, ...commands]) expect(stored).not.toContain(value);
}
async function history(page: Page): Promise<Locator> {
  // Keep the locator live while an asynchronously restored mobile selection
  // mounts its sheet; the sheet's mirrored control follows the page header.
  await page.getByTestId('definition-history').last().click();
  const view = page.getByRole('dialog').filter({ has: page.getByTestId('definition-receipt') });
  await expect(view).toBeVisible();
  return view;
}

for (const locale of ['en', 'es'] as const) {
  test(`native owner definition writes, lost acknowledgement, history and reload (${locale})`, async ({ page }) => {
    test.skip(!ownerUrl || !home || !binary, 'Requires Rust owner_definition_browser fixture');
    const words = locale === 'en' ? english : spanish;
    const sent: Sent[] = []; const methods: string[] = []; const failures: string[] = [];
    const actionId = `definition-action-${locale}`; const eventId = `definition-event-${locale}`;
    const actionName = `Committed definition action ${locale}`; const eventName = `Committed definition event ${locale}`;
    let armed = true; let dropId: string | number | undefined;
    let held: { page: WebSocketRoute; server: WebSocketRoute; record: NativeRecord } | undefined;
    await page.routeWebSocket('**/*', (socket) => {
      if (new URL(socket.url()).host !== new URL(ownerUrl!).host) { failures.push('External WebSocket'); socket.close(); return; }
      const server = socket.connectToServer();
      socket.onMessage((message) => {
        const request = JSON.parse(String(message)) as Sent;
        if (typeof request.method === 'string') methods.push(request.method);
        if (request.method === 'automation.definition.submit') {
          expect(request.meta).toBeUndefined(); sent.push(request);
          if (armed && request.params['operation'] === 'update' && target(request) === actionId) { armed = false; dropId = request.id; }
        }
        server.send(message);
      });
      server.onMessage((message) => {
        const frame = JSON.parse(String(message)) as { response?: { id: string | number; result?: { state?: NativeState } } };
        if (dropId !== undefined && frame.response?.id === dropId) {
          const record = frame.response.result?.state?.recent[0];
          if (!record || record.outcome !== 'succeeded') throw new Error('The real commit did not produce a verified native receipt');
          held = { page: socket, server, record }; dropId = undefined;
          // Hold the real successful frame until newer local text has been
          // entered, then close transport instead of returning that frame.
          return;
        }
        socket.send(message);
      });
    });
    page.on('pageerror', (error) => failures.push(error.message));
    page.on('request', (request) => { const url = new URL(request.url()); if (!['blob:', 'data:'].includes(url.protocol) && url.host !== new URL(ownerUrl!).host) failures.push('External request'); });
    await page.setViewportSize(locale === 'es' ? { width: 390, height: 844 } : { width: 1360, height: 960 });
    await page.addInitScript((language) => localStorage.setItem('tiktok-live-locale', language), locale);
    if (cookies.length) await page.context().addCookies(cookies);
    await page.goto(ownerUrl!);
    if (!cookies.length) { await page.getByLabel('Pairing code', { exact: true }).fill('948231'); await page.getByRole('button', { name: 'Connect', exact: true }).click(); }
    await page.getByRole('link', { name: words.navigation.settings, exact: true }).waitFor({ state: 'attached' });
    cookies = await page.context().cookies();
    await page.goto(`${ownerUrl!}/#/automations`);
    await select(page, 'action', `Owner definition action ${locale}`, words);
    const name = editor(page, 'action').locator('input[type="text"]').first();
    await name.fill(actionName);
    await editor(page, 'action').getByRole('button', { name: words.automations.editor.save, exact: true }).click();
    await expect.poll(() => held !== undefined).toBe(true);
    const original = held!.record;
    await name.fill(`Newer local draft ${locale}`);
    await Promise.all([held!.page.close({ code: 1012, reason: 'fixture discards committed reply' }), held!.server.close()]);
    await expect(name).toHaveValue(`Newer local draft ${locale}`);
    let view = await history(page);
    await expect(view.getByTestId('definition-receipt').filter({ hasText: original.intent.commandId })).toHaveCount(1);
    expect((await completed(sent[0]!)).intent.commandId).toBe(original.intent.commandId);
    expect((await rpc('automation.get', { kind: 'action', id: actionId }))['name']).toBe(actionName);
    await closeDialog(view, words);
    await expect(name).toHaveValue(`Newer local draft ${locale}`);
    await expect(editor(page, 'action').getByRole('button', { name: words.automations.editor.save, exact: true })).toBeDisabled();
    expect(sent).toHaveLength(1);
    await storageIsPrivate(page, locale, [original.intent.commandId]);

    await page.reload();
    await select(page, 'action', actionName, words);
    view = await history(page);
    await expect(view.getByTestId('definition-receipt').filter({ hasText: original.intent.commandId })).toHaveCount(1);
    await mkdir(shots, { recursive: true });
    await view.screenshot({ path: `${shots}/owner-definition-${locale}-recovery.png` });
    expect(await view.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    await closeDialog(view, words);
    const commands = [original.intent.commandId];
    await backToLibrary(page, words);
    for (const enabled of [true, false]) {
      const before = sent.length;
      const row = page.locator('.automation-library tbody tr').filter({ hasText: actionName });
      await row.getByRole('checkbox').click();
      const confirmation = page.getByRole('dialog', { name: words.automations.conflict.toggleTitle, exact: true });
      await confirmation.getByRole('button', { name: words.automations.conflict.confirmToggle, exact: true }).click();
      await expect(confirmation).not.toBeVisible();
      commands.push((await completed(await submitted(sent, before))).intent.commandId);
      if (enabled) await expect(row.getByRole('checkbox')).toBeChecked();
      else await expect(row.getByRole('checkbox')).not.toBeChecked();
      await backToLibrary(page, words);
    }
    await select(page, 'event', `Owner definition event ${locale}`, words);
    const eventBefore = sent.length;
    await editor(page, 'event').locator('input[type="text"]').first().fill(eventName);
    await editor(page, 'event').getByRole('button', { name: words.automations.editor.save, exact: true }).click();
    commands.push((await completed(await submitted(sent, eventBefore))).intent.commandId);
    await settledEditor(page, words);
    for (const [kind, id, label] of [['action', actionId, actionName], ['event', eventId, eventName]] as const) {
      await select(page, kind, label, words);
      const before = sent.length;
      await editor(page, kind).getByRole('button', { name: words.automations.editor.delete, exact: true }).click();
      const confirmation = page.getByRole('dialog', { name: kind === 'action' ? words.automations.editor.deleteActionTitle : words.automations.editor.deleteEventTitle, exact: true });
      await confirmation.getByRole('button', { name: words.automations.editor.deleteConfirm, exact: true }).click();
      await expect(confirmation).not.toBeVisible();
      const record = await completed(await submitted(sent, before)); commands.push(record.intent.commandId);
      await settledEditor(page, words);
      expect(record.intent.id).toBe(id);
      if (kind === 'action') {
        expect(record.receipt?.affectedResources).toBe(2);
        expect((await rpc('automation.get', { kind: 'event', id: eventId }))['actionIds']).toEqual([]);
      }
    }
    for (const kind of ['event', 'action'] as const) {
      const before = sent.length;
      await backToLibrary(page, words);
      await page.getByRole('tab', { name: new RegExp(`^${kind === 'action' ? words.automations.library.actions : words.automations.library.events} \\(`) }).click();
      await page.getByRole('button', { name: kind === 'action' ? words.automations.library.newAction : words.automations.library.newEvent, exact: true }).click();
      if (kind === 'action') {
        const picker = page.getByRole('dialog', { name: words.automations.picker.title, exact: true });
        await picker.getByRole('searchbox').fill('core.log');
        await picker.locator('.type-picker__card').click();
      }
      const createdName = `Native created ${kind} ${locale}`;
      await editor(page, kind).locator('input[type="text"]').first().fill(createdName);
      await editor(page, kind).getByRole('button', { name: words.automations.editor.save, exact: true }).click();
      const request = await submitted(sent, before); const record = await completed(request); commands.push(record.intent.commandId);
      await settledEditor(page, words);
      expect(request.params['operation']).toBe('create'); expect(target(request)).toMatch(kind === 'action' ? /^act-/ : /^evt-/);
      expect((await rpc('automation.get', { kind, id: target(request) }))['name']).toBe(createdName);
    }
    expect(sent).toHaveLength(8); expect(new Set(commands).size).toBe(8);
    expect(methods.filter((method) => /^automation\.(create|update|delete|enable|disable)$/.test(method))).toEqual([]);
    await storageIsPrivate(page, locale, commands);
    await page.reload(); view = await history(page);
    for (const command of commands) await expect(view.getByTestId('definition-receipt').filter({ hasText: command })).toHaveCount(1);
    expect(JSON.stringify(await rpc('automation.definition.get'))).not.toContain(`definition-source-${locale}`);
    expect(failures).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1)).toBe(true);
  });
}
