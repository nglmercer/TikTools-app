/** Actual packaged Vue → local adapter → IPC. A real committed reply is held
 * while the candidate changes, then discarded; no host result is fabricated. */
import { expect, test, type BrowserContext, type Locator, type Page, type WebSocketRoute } from '@playwright/test';
import { execFile } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import { promisify } from 'node:util';
import english from '../../src/web/shared/i18n/locales/en.ts';
import spanish from '../../src/web/shared/i18n/locales/es.ts';

const execute = promisify(execFile);
const ownerUrl = process.env['TIKTOOLS_OWNER_LIBRARY_URL'];
const home = process.env['TIKTOOLS_OWNER_LIBRARY_HOME'];
const binary = process.env['TIKTOOLS_OWNER_LIBRARY_BINARY'];
const shots = '/home/meme/.cache/tiktools-test-runs';
type Json = Record<string, unknown>;
type Words = typeof english | typeof spanish;
interface Sent { id: string | number; method: string; params: Json; meta?: unknown }
interface Intent { commandId: string; operation: string; target: string; expectedRevision: string; definitionDigest: string; hostInstanceId: string }
interface NativeRecord { intent: Intent; outcome: string; receipt: { revision: string; affectedResources: number } | null }
interface NativeState { hostId: string; hostInstanceId: string; pending: Intent | null; recent: NativeRecord[] }
interface Snapshot { catalog: { packs: Array<{ id: string; name: string; eventIds: string[]; actionIds: string[] }>; activeId: string; templates: Json[] }; definitions: { actions: Json[]; events: Json[] } }
let cookies: Awaited<ReturnType<BrowserContext['cookies']>> = [];

async function rpc<T = Json>(method: string, params: Json = {}): Promise<T> {
  if (!home || !binary) throw new Error('Real native library fixture is unavailable');
  const env: Record<string, string | undefined> = { ...process.env, TIKTOOLS_HOME: home, TIKTOOLS_CLOUD_ORIGIN: '' };
  for (const [key, folder] of Object.entries({ TIKTOOLS_DATA_DIR: 'data', TIKTOOLS_PLUGINS_DIR: 'plugins', TIKTOOLS_PLUGIN_DATA_DIR: 'plugin-data',
    TIKTOOLS_BUILTIN_PLUGINS_DIR: 'builtins', TIKTOOLS_DEV_PLUGINS_DIR: 'dev-plugins', TIKTOOLS_LOG_DIR: 'logs', TIKTOOLS_TEMP_DIR: 'temp' })) env[key] = `${home}/${folder}`;
  const output = await execute(binary, ['--json', 'api', 'call', method, JSON.stringify(params), '--confirm'], { cwd: home, env, maxBuffer: 9 * 1024 * 1024 });
  return JSON.parse(output.stdout) as T;
}
function target(mutation: Json): string {
  switch (mutation['operation']) {
    case 'apply-template': return mutation['profileId'] as string;
    case 'import-profile': return (mutation['profile'] as Json)['id'] as string;
    case 'import-templates': return 'templates';
    default: return mutation['id'] as string;
  }
}
async function submitted(sent: Sent[], before: number, operation: string): Promise<Sent> {
  await expect.poll(() => sent.length).toBe(before + 1);
  const request = sent[before]!;
  expect((request.params['mutation'] as Json)['operation']).toBe(operation);
  return request;
}
async function completed(request: Sent): Promise<NativeRecord> {
  let found: NativeRecord | undefined;
  const mutation = request.params['mutation'] as Json;
  await expect.poll(async () => {
    const state = await rpc<NativeState>('automation.library.get');
    found = state.recent.find((row) => row.intent.operation === mutation['operation'] && row.intent.target === target(mutation) &&
      row.intent.expectedRevision === request.params['expectedRevision'] && row.intent.definitionDigest === request.params['definitionDigest'] && row.intent.hostInstanceId === request.params['hostInstanceId']);
    return { pending: state.pending, outcome: found?.outcome ?? null };
  }).toEqual({ pending: null, outcome: 'succeeded' });
  return found!;
}
const review = (page: Page, words: Words) => page.getByRole('dialog', { name: words.libraryRecovery.reviewTitle, exact: true });
async function closeDialog(dialog: Locator, words: Words): Promise<void> {
  await dialog.locator('header').getByRole('button', { name: words.common.close, exact: true }).click();
  await expect(dialog).not.toBeVisible();
}
async function finishNavigation(page: Page, words: Words, finished: () => Promise<boolean>): Promise<void> {
  const discard = page.getByRole('dialog', { name: words.automations.editor.discardTitle, exact: true });
  await expect.poll(async () => await discard.isVisible() || await finished()).toBe(true);
  if (await discard.isVisible()) {
    await discard.getByRole('button', { name: words.automations.editor.discardConfirm, exact: true }).click();
    await expect(discard).not.toBeVisible();
  }
  await expect.poll(finished).toBe(true);
}
async function backToLibrary(page: Page, words: Words): Promise<void> {
  const sheet = page.locator('.automations-page__sheet');
  if (await sheet.isVisible()) {
    await sheet.getByRole('button', { name: words.common.back, exact: true }).click();
    await finishNavigation(page, words, async () => !await sheet.isVisible());
  }
}
async function menu(page: Page, words: Words, item: string): Promise<void> {
  await backToLibrary(page, words);
  await page.getByRole('button', { name: words.automations.profile.menu, exact: true }).click();
  await page.getByRole('menuitem', { name: item, exact: true }).click();
}
async function confirm(page: Page, words: Words, sent: Sent[], operation: string): Promise<{ request: Sent; record: NativeRecord }> {
  const dialog = review(page, words); await expect(dialog).toBeVisible();
  await expect(dialog.getByTestId('library-confirm')).toBeEnabled();
  const before = sent.length; await dialog.getByTestId('library-confirm').click();
  const request = await submitted(sent, before, operation); const record = await completed(request);
  await expect(dialog).not.toBeVisible();
  return { request, record };
}
async function history(page: Page, words: Words): Promise<Locator> {
  // Reload mounts metadata readers while the coherent source snapshot arrives.
  // Open recovery after that boot phase, rather than racing its busy transition.
  await expect(page.getByRole('button', { name: words.automations.profile.menu, exact: true })).toBeEnabled();
  await expect(page.getByTestId('library-loading')).toHaveCount(0);
  await page.getByTestId('library-history').last().click();
  const dialog = page.getByRole('dialog', { name: words.libraryRecovery.title, exact: true });
  await expect(dialog).toBeVisible(); return dialog;
}
function documentTemplate(locale: string, id: string, name: string): Json {
  return { templateVersion: 1, id, title: { default: name }, description: { default: 'Private browser fixture' }, icon: 'plugin',
    actions: [{ name: `Imported library action ${locale}`, typeId: 'core.log', enabled: false, config: { message: `private-library-profile-source-${locale}`, float: 1, tiny: 0.000001 } }],
    event: { name: `Imported library event ${locale}`, enabled: false, trigger: 'tiktok.chat', filters: [], cooldownMs: 0, cooldownScope: 'global', runMode: 'all' } };
}
async function file(dialog: Locator, value: Json, name: string): Promise<void> {
  await dialog.locator('input[type="file"]').setInputFiles({ name, mimeType: 'application/json', buffer: Buffer.from(JSON.stringify(value)) });
}
async function storageIsPrivate(page: Page, locale: string, commands: string[]): Promise<void> {
  const stored = await page.evaluate(() => JSON.stringify({ local: Object.entries(localStorage), session: Object.entries(sessionStorage) }));
  for (const marker of ['owner-library-', 'private-library-', `Newer library candidate ${locale}`, ...commands]) expect(stored).not.toContain(marker);
}

for (const locale of ['en', 'es'] as const) {
  test(`atomic library operations, lost reply, newer candidate and reload (${locale})`, async ({ page }) => {
    test.skip(!ownerUrl || !home || !binary, 'Requires Rust owner_library_browser fixture');
    const words = locale === 'en' ? english : spanish;
    const sent: Sent[] = []; const all: Sent[] = []; const failures: string[] = []; const commands: string[] = [];
    let armed = true; let dropId: string | number | undefined;
    let closedOriginal = false; let reconnectedReply = false;
    let held: { page: WebSocketRoute; server: WebSocketRoute; record: NativeRecord } | undefined;
    await page.routeWebSocket('**/*', (socket) => {
      if (new URL(socket.url()).host !== new URL(ownerUrl!).host) { failures.push('External WebSocket'); socket.close(); return; }
      const server = socket.connectToServer();
      socket.onMessage((message) => {
        const request = JSON.parse(String(message)) as Sent;
        if (typeof request.method === 'string') all.push(request);
        if (request.method === 'automation.library.submit') {
          expect(request.meta).toBeUndefined(); sent.push(request);
          if (armed && (request.params['mutation'] as Json)['operation'] === 'apply-template') { armed = false; dropId = request.id; }
        }
        server.send(message);
      });
      server.onMessage((message) => {
        const frame = JSON.parse(String(message)) as { response?: { id: string | number; result?: { state?: NativeState } } };
        if (dropId !== undefined && frame.response?.id === dropId) {
          const record = frame.response.result?.state?.recent[0];
          if (!record || record.outcome !== 'succeeded') throw new Error('Real library commit did not yield its original receipt');
          held = { page: socket, server, record }; dropId = undefined; return;
        }
        if (closedOriginal && frame.response?.result) reconnectedReply = true;
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
    cookies = await page.context().cookies(); await page.goto(`${ownerUrl!}/#/automations`);
    await expect(page.locator('.profile-toolbar').getByRole('combobox')).toBeVisible();
    await expect(page.getByRole('button', { name: words.automations.profile.menu, exact: true })).toBeEnabled();
    await expect(page.getByTestId('library-loading')).toHaveCount(0);

    await menu(page, words, words.automations.profile.create);
    const create = page.getByRole('dialog', { name: words.automations.profile.createTitle, exact: true });
    await create.getByLabel(words.automations.profile.createName, { exact: true }).fill(`Live library ${locale}`);
    await create.getByRole('button', { name: words.automations.profile.createConfirm, exact: true }).click();
    commands.push((await confirm(page, words, sent, 'create-profile')).record.intent.commandId);

    await page.getByRole('button', { name: words.automations.templates.open, exact: true }).click();
    let templates = page.getByRole('dialog', { name: words.automations.templates.title, exact: true });
    await templates.getByRole('searchbox').fill(`Library seed ${locale}`);
    await templates.locator('.rule-templates__card').filter({ hasText: `Library seed ${locale}` }).click();
    let configure = page.getByRole('dialog', { name: `Library seed ${locale}`, exact: true });
    await configure.getByRole('button', { name: words.automations.templates.create, exact: true }).click();
    const draft = review(page, words); await expect(draft).toBeVisible();
    const before = sent.length; await draft.getByTestId('library-confirm').click();
    const lost = await submitted(sent, before, 'apply-template');
    await expect.poll(() => held !== undefined).toBe(true);
    const original = held!.record; const submittedEntry = (lost.params['mutation'] as Json)['entry'] as Json;
    const submittedEvent = submittedEntry['event'] as Json; const eventId = submittedEvent['id'] as string;
    const actionIds = (submittedEntry['actions'] as Json[]).map((a) => a['id'] as string);
    expect(eventId).toMatch(/^evt-/); for (const id of actionIds) expect(id).toMatch(/^act-/);
    const candidate = draft.locator('[data-test="library-record-name"][data-kind="event"]');
    await candidate.fill(`Newer library candidate ${locale}`);
    expect((await rpc('automation.get', { kind: 'event', id: eventId }))['name']).toBe(submittedEvent['name']);
    await Promise.all([held!.page.close({ code: 1012, reason: 'fixture discards committed library reply' }), held!.server.close()]);
    closedOriginal = true;
    await expect(draft.getByTestId('library-read-current')).toBeEnabled();
    await expect.poll(() => reconnectedReply).toBe(true);
    await expect(candidate).toHaveValue(`Newer library candidate ${locale}`);
    await expect(draft.getByTestId('library-confirm')).toBeDisabled();
    await draft.getByTestId('library-read-current').click();
    await expect(draft.getByTestId('library-current-review')).toContainText(submittedEvent['name'] as string);
    await expect(draft.getByTestId('library-use-current')).toHaveCount(0);
    await draft.getByRole('button', { name: words.libraryRecovery.history, exact: true }).click();
    let view = page.getByRole('dialog', { name: words.libraryRecovery.title, exact: true });
    await expect(view.getByTestId('library-receipt').filter({ hasText: original.intent.commandId })).toHaveCount(1);
    expect((await completed(lost)).intent.commandId).toBe(original.intent.commandId); commands.push(original.intent.commandId);
    await closeDialog(view, words); await expect(candidate).toHaveValue(`Newer library candidate ${locale}`);
    expect(sent).toHaveLength(2); await storageIsPrivate(page, locale, commands);
    await draft.getByTestId('library-dismiss').click(); await expect(draft).not.toBeVisible();
    configure = page.getByRole('dialog', { name: `Library seed ${locale}`, exact: true }); await closeDialog(configure, words);
    await page.reload(); view = await history(page, words);
    await expect(view.getByTestId('library-receipt').filter({ hasText: original.intent.commandId })).toHaveCount(1);
    await closeDialog(view, words); expect(sent).toHaveLength(2);
    for (const id of actionIds) expect((await rpc('automation.get', { kind: 'action', id }))['config']).toHaveProperty('message', `private-library-seed-source-${locale}`);
    expect((await rpc('automation.get', { kind: 'event', id: eventId }))['name']).toBe(submittedEvent['name']);

    await menu(page, words, words.automations.profile.import);
    const importer = page.getByRole('dialog', { name: words.automations.import.profileTitle, exact: true });
    const profileId = `browser-import-${locale}`; const profileName = `Imported library profile ${locale}`;
    // One bounded source file expands past the former local WebSocket ceiling.
    // Unicode and JSON escapes also exercise byte budgets and canonical hashing.
    const marker = `private-library-profile-source-${locale}:ñ\\\n`;
    const largeSource = marker.repeat(Math.ceil(180 * 1024 / Buffer.byteLength(JSON.stringify(marker).slice(1, -1))));
    const profileTemplate = documentTemplate(locale, `profile-template-${locale}`, `Profile template ${locale}`);
    profileTemplate['actions'] = Array.from({ length: 6 }, (_, index) => ({ name: `Imported library action ${locale} ${index + 1}`,
      typeId: 'core.log', enabled: false, config: { message: '{{params.big}}', float: 1, tiny: 0.000001 } }));
    const profileDocument = { profileVersion: 1, id: profileId, name: profileName, description: `private-library-profile-source-${locale}`,
      params: { big: largeSource }, templates: [{ template: profileTemplate, params: {} }] };
    expect(Buffer.byteLength(JSON.stringify(profileDocument))).toBeLessThanOrEqual(256 * 1024);
    await file(importer, profileDocument, `profile-${locale}.json`);
    await importer.getByRole('button', { name: words.automations.import.profileApply.replace('{count}', '1'), exact: true }).click();
    const importedProfile = await confirm(page, words, sent, 'import-profile');
    expect(Buffer.byteLength(JSON.stringify(importedProfile.request.params))).toBeGreaterThan(1024 * 1024);
    commands.push(importedProfile.record.intent.commandId);
    await expect(importer).not.toBeVisible();
    await expect(page.locator('.profile-toolbar').getByRole('combobox')).toContainText(profileName);
    const largeSnapshot = await rpc<Snapshot>('automation.library.snapshot');
    expect(Buffer.byteLength(JSON.stringify(largeSnapshot))).toBeGreaterThan(1024 * 1024);
    const importedActions = (largeSnapshot.catalog.packs.find((pack) => pack.id === profileId))!.actionIds;
    expect(importedActions).toHaveLength(6);
    for (const id of importedActions) expect(largeSnapshot.definitions.actions.find((row) => row['id'] === id)?.['config']).toEqual({ message: largeSource, float: 1, tiny: 0.000001 });

    const actionsTab = page.getByRole('tab', { name: new RegExp(`^${words.automations.library.actions} \\(`) });
    await actionsTab.click(); await finishNavigation(page, words, async () => await actionsTab.getAttribute('aria-selected') === 'true');
    await page.getByRole('button', { name: words.automations.library.newAction, exact: true }).click();
    const picker = page.getByRole('dialog', { name: words.automations.picker.title, exact: true });
    await picker.getByRole('searchbox').fill('core.log'); await picker.locator('.type-picker__card').click();
    const action = page.locator('.action-editor:visible'); const assignedName = `Assigned library action ${locale}`;
    await action.locator('input[type="text"]').first().fill(assignedName);
    await action.getByRole('button', { name: words.automations.editor.save, exact: true }).click();
    const assignment = await confirm(page, words, sent, 'assign-record'); commands.push(assignment.record.intent.commandId);
    const assignedId = (assignment.request.params['mutation'] as Json)['id'] as string;
    expect((await rpc<Snapshot>('automation.library.snapshot')).catalog.packs.find((p) => p.id === profileId)?.actionIds).toContain(assignedId);
    await backToLibrary(page, words);

    await page.getByRole('button', { name: words.automations.templates.open, exact: true }).click();
    templates = page.getByRole('dialog', { name: words.automations.templates.title, exact: true });
    await templates.getByRole('button', { name: words.automations.templates.import, exact: true }).click();
    const templateImport = page.getByRole('dialog', { name: words.automations.templates.importTitle, exact: true });
    const customId = `browser-gallery-${locale}`; const customName = `Browser custom library ${locale}`;
    await file(templateImport, documentTemplate(locale, customId, customName), `template-${locale}.json`);
    await templateImport.getByRole('button', { name: words.automations.import.run, exact: true }).click();
    commands.push((await confirm(page, words, sent, 'import-templates')).record.intent.commandId);
    await templateImport.getByRole('button', { name: words.automations.templates.back, exact: true }).click();
    templates = page.getByRole('dialog', { name: words.automations.templates.title, exact: true });
    await templates.getByRole('searchbox').fill(customName);
    await templates.locator('.rule-templates__card').filter({ hasText: customName }).click();
    configure = page.getByRole('dialog', { name: customName, exact: true });
    await configure.getByRole('button', { name: words.automations.templates.delete, exact: true }).click();
    commands.push((await confirm(page, words, sent, 'delete-template')).record.intent.commandId);
    templates = page.getByRole('dialog', { name: words.automations.templates.title, exact: true }); await closeDialog(templates, words);
    expect((await rpc<Snapshot>('automation.library.snapshot')).catalog.templates.some((t) => t['id'] === customId)).toBe(false);

    const combo = page.locator('.profile-toolbar').getByRole('combobox'); await combo.click();
    await page.getByRole('option', { name: new RegExp(`^Delete library profile ${locale} ·`) }).click();
    await finishNavigation(page, words, async () => await review(page, words).isVisible());
    commands.push((await confirm(page, words, sent, 'switch-profile')).record.intent.commandId);
    await expect(combo).toContainText(`Delete library profile ${locale}`);
    await menu(page, words, words.automations.profile.delete);
    const deletion = page.getByRole('dialog', { name: words.automations.profile.deleteTitle, exact: true });
    await deletion.getByRole('button', { name: words.automations.profile.deleteConfirm, exact: true }).click();
    await expect(review(page, words).getByTestId('library-review-records')).toContainText(`external library event ${locale}`);
    const removed = await confirm(page, words, sent, 'delete-profile'); commands.push(removed.record.intent.commandId);
    expect(removed.record.receipt?.affectedResources).toBe(4);
    expect((await rpc('automation.get', { kind: 'event', id: `library-external-event-${locale}` }))['actionIds']).toEqual([]);
    const saved = await rpc<Snapshot>('automation.library.snapshot');
    expect(saved.catalog.packs.some((p) => p.id === `delete-library-${locale}`)).toBe(false);
    expect(saved.definitions.actions.some((r) => r['id'] === `library-delete-action-${locale}`)).toBe(false);
    expect(saved.definitions.events.some((r) => r['id'] === `library-delete-event-${locale}`)).toBe(false);
    expect(sent).toHaveLength(8); expect(new Set(commands).size).toBe(8);
    expect(all.filter((r) => /^automation\.(create|update|delete|enable|disable|set_enablement)$/.test(r.method))).toEqual([]);
    expect(all.filter((r) => r.method === 'app.state.set' && ['behavior.profiles.packs', 'behavior.profiles.active', 'behavior.templates.custom'].includes(r.params['key'] as string))).toEqual([]);
    await storageIsPrivate(page, locale, commands);
    await page.reload(); view = await history(page, words);
    for (const command of commands) await expect(view.getByTestId('library-receipt').filter({ hasText: command })).toHaveCount(1);
    await mkdir(shots, { recursive: true }); await view.screenshot({ path: `${shots}/owner-library-${locale}-recovery.png` });
    expect(await view.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
    expect(JSON.stringify(await rpc('automation.library.get'))).not.toContain('private-library-');
    expect(failures).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth + 1)).toBe(true);
  });
}
