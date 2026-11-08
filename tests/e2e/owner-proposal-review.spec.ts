/** Actual packaged Vue → local browser adapter → IPC → one owner Core.
 * Invoked by the opt-in Rust owner_proposal_browser fixture; never installs a fake host. */
import { expect, test, type BrowserContext, type Locator, type Page } from '@playwright/test';
import { execFile } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import { promisify } from 'node:util';
import type { AdoptionState, ProposalReview } from '../../src/web/features/review-automation-proposal/model/contracts.ts';

const execute = promisify(execFile);
const ownerUrl = process.env['TIKTOOLS_OWNER_PROPOSAL_URL'];
const home = process.env['TIKTOOLS_OWNER_PROPOSAL_HOME'];
const binary = process.env['TIKTOOLS_OWNER_PROPOSAL_BINARY'];
const shots = '/home/meme/.cache/tiktools-test-runs';
type Json = Record<string, unknown>;
interface SentRequest { method: string; params: Json; meta?: unknown }
interface ReviewEnvelope { result: ProposalReview }
// The local adapter's pairing code is one-use. Share its isolated test cookie
// in this worker's memory only; never persist credentials to a storage file.
let ownerCookies: Awaited<ReturnType<BrowserContext['cookies']>> = [];

async function rpc<T = Json>(method: string, params: Json = {}, metadata = false): Promise<T> {
  if (!home || !binary) throw new Error('Real owner fixture is unavailable');
  const env: Record<string, string | undefined> = { ...process.env, TIKTOOLS_HOME: home, TIKTOOLS_CLOUD_ORIGIN: '' };
  for (const [key, directory] of Object.entries({
    TIKTOOLS_DATA_DIR: 'data', TIKTOOLS_PLUGINS_DIR: 'plugins', TIKTOOLS_PLUGIN_DATA_DIR: 'plugin-data',
    TIKTOOLS_BUILTIN_PLUGINS_DIR: 'builtins', TIKTOOLS_DEV_PLUGINS_DIR: 'dev-plugins',
    TIKTOOLS_LOG_DIR: 'logs', TIKTOOLS_TEMP_DIR: 'temp',
  })) env[key] = `${home}/${directory}`;
  const args = ['--json', 'api', 'call', method, JSON.stringify(params), '--confirm'];
  if (metadata) args.push('--include-meta');
  const output = await execute(binary, args, { cwd: home, env, maxBuffer: 8 * 1024 * 1024 });
  return JSON.parse(output.stdout) as T;
}

const messages = {
  en: { open: 'Review saved authority', history: 'Adoption history', read: 'Read saved source again', close: 'Close',
    delegated: 'Delegated authority', changed: 'The saved source or a referenced action changed. Read and review it again before adoption.',
    success: 'Adopted; disabled at that time', absent: 'No saved record exists at this ID.', actions: /^Actions/ },
  es: { open: 'Revisar autoridad guardada', history: 'Historial de adopciones', read: 'Volver a leer el código guardado', close: 'Cerrar',
    delegated: 'Autoridad delegada', changed: 'Cambió la definición guardada o una acción referenciada. Vuelve a leerla y revisarla antes de adoptar.',
    success: 'Adoptada; desactivada en ese momento', absent: 'No existe un registro guardado con este ID.', actions: /^Acciones/ },
};

function reviewDialog(page: Page): Locator {
  return page.getByRole('dialog').filter({ has: page.locator('.proposal-journal') });
}
async function openReview(page: Page, label: string): Promise<Locator> {
  const sheet = page.locator('.automations-page__sheet');
  const scope = await sheet.isVisible() ? sheet : page;
  await scope.getByRole('button', { name: label, exact: true }).filter({ visible: true }).first().click();
  const dialog = reviewDialog(page);
  await expect(dialog.locator('.reviewed-definition').first().locator('pre')).toBeVisible();
  return dialog;
}
async function noBrowserJournal(page: Page, locale: string, commands: string[]): Promise<void> {
  const storage = await page.evaluate(() => JSON.stringify({ local: Object.entries(localStorage), session: Object.entries(sessionStorage) }));
  for (const forbidden of [`proposal-source-${locale}`, `proposal-script-${locale}`, `Unsaved owner draft ${locale}`, 'owner-adopt-', ...commands]) {
    expect(storage).not.toContain(forbidden);
  }
}
async function noHorizontalOverflow(dialog: Locator): Promise<void> {
  expect(await dialog.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
  for (const button of await dialog.locator('button:visible .base-button__label').all()) {
    expect(await button.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
  }
}

for (const locale of ['en', 'es'] as const) {
  test(`real owner proposal review and native adoption (${locale})`, async ({ page }) => {
    test.skip(!ownerUrl || !home || !binary, 'Requires the real Rust owner_proposal_browser fixture');
    const words = messages[locale]; const id = `proposal-${locale}`; const branch = `disabled-${locale}`;
    const submitted: SentRequest[] = []; const methods: string[] = []; const failures: string[] = [];
    const origin = new URL(ownerUrl!).host;
    page.on('pageerror', () => failures.push('Owner proposal page error'));
    page.on('request', (request) => {
      const url = new URL(request.url());
      if (!['blob:', 'data:'].includes(url.protocol) && url.host !== origin) failures.push('Unexpected external owner request');
    });
    page.on('websocket', (socket) => socket.on('framesent', (frame) => {
      try {
        const value = JSON.parse(String(frame.payload)) as SentRequest;
        if (typeof value['method'] === 'string') methods.push(value['method']);
        if (value['method'] === 'automation.adoption.submit') submitted.push(value);
      } catch { /* Transport pings are not RPC calls. */ }
    }));
    await page.setViewportSize(locale === 'es' ? { width: 390, height: 844 } : { width: 1360, height: 960 });
    await page.addInitScript((language) => { localStorage.setItem('tiktok-live-locale', language); }, locale);
    if (ownerCookies.length) await page.context().addCookies(ownerCookies);
    await page.goto(ownerUrl!);
    if (!ownerCookies.length) {
      await page.getByLabel('Pairing code', { exact: true }).fill('948231');
      await page.getByRole('button', { name: 'Connect', exact: true }).click();
    }
    await page.getByRole('link', { name: locale === 'es' ? 'Ajustes' : 'Settings', exact: true }).waitFor({ state: 'attached' });
    ownerCookies = await page.context().cookies();
    await page.goto(`${ownerUrl!}/#/automations`);
    await page.locator('.automation-library tbody tr').filter({ hasText: `Moderator event ${locale}` }).click();
    const editorName = page.locator('.event-editor:visible input[type="text"]').first();
    await editorName.fill(`Unsaved owner draft ${locale}`);
    let dialog = await openReview(page, words.open);
    const initial = await rpc<ReviewEnvelope>('automation.review', { kind: 'event', id }, true);
    const source = initial.result;
    const record = source.record;
    if (!record) throw new Error('The real proposal must have a saved definition');
    expect(JSON.parse(await dialog.locator('.reviewed-definition').first().locator('pre').innerText())).toEqual(source['record']);
    expect(record['enabled']).toBe(false);
    expect((record['provenance'] as { authority: { kind: string } }).authority.kind).toBe('owner');
    const authority = source.provenance?.authority;
    expect(authority?.kind).toBe('delegated');
    if (!authority || authority.kind !== 'delegated') throw new Error('Proposal sidecar is not delegated');
    expect(source.dependencies.map((dependency) => dependency.id)).toEqual([id, branch, `missing-${locale}`]);
    await expect(dialog.locator('.reviewed-definition').first()).toContainText(words.delegated);
    await expect(dialog).toContainText(authority.principalId);
    await expect(dialog).toContainText(authority.sessionId);
    const dependencies = dialog.locator('.proposal-dependencies details');
    await expect(dependencies).toHaveCount(3);
    for (const dependency of await dependencies.all()) await dependency.locator('summary').click();
    for (let index = 0; index < 2; index += 1) {
      expect(JSON.parse(await dependencies.nth(index).locator('pre').innerText())).toEqual(source.dependencies[index]!.record);
      await expect(dependencies.nth(index)).toContainText(words.delegated);
    }
    await expect(dependencies.nth(2)).toContainText(words.absent);
    await noHorizontalOverflow(dialog);
    await mkdir(shots, { recursive: true });
    await dialog.locator('.base-dialog__body').evaluate((body) => { body.scrollTop = 0; });
    await page.screenshot({ path: `${shots}/owner-proposal-review-${locale}-${locale === 'es' ? '390' : 'desktop'}.png` });
    await dependencies.nth(1).scrollIntoViewIfNeeded();
    await page.screenshot({ path: `${shots}/owner-proposal-dependencies-${locale}-${locale === 'es' ? '390' : 'desktop'}.png` });
    const acknowledge = dialog.locator('[data-test="adoption-acknowledge"]');
    const adopt = dialog.locator('[data-test="adoption-submit"]');
    await expect(adopt).toBeDisabled(); await acknowledge.check(); await expect(adopt).toBeEnabled();
    const changedBranch = source.dependencies[1]!.record;
    if (!changedBranch) throw new Error('The disabled branch must have a saved definition');
    await rpc('automation.update', { kind: 'action', id: branch, record: { ...changedBranch, name: `Changed branch ${locale}` } });
    // Either the pushed revision invalidates consent first or submit's fresh
    // review detects the change. Neither path may adopt a newer unseen graph.
    await adopt.evaluate((button: HTMLButtonElement) => { if (!button.disabled) button.click(); });
    await expect(dialog).toContainText(words.changed);
    await expect(adopt).toBeDisabled(); await expect(acknowledge).not.toBeChecked();
    expect(submitted).toHaveLength(0);
    expect((await rpc<AdoptionState>('automation.adoption.get')).recent.filter((entry) => entry.intent.id === id)).toHaveLength(0);
    expect(JSON.parse(await dialog.locator('.reviewed-definition').first().locator('pre').innerText())).toEqual(source['record']);
    await dialog.getByRole('button', { name: words.read, exact: true }).click();
    await expect(dialog).toContainText(`Changed branch ${locale}`);
    await expect(acknowledge).not.toBeChecked(); await acknowledge.check(); await adopt.click();
    await expect(dialog.locator('.proposal-history')).toContainText(words.success);
    const afterEvent = await rpc<AdoptionState>('automation.adoption.get');
    const eventRecord = afterEvent.recent.find((entry) => entry.intent.kind === 'event' && entry.intent.id === id);
    if (!eventRecord) throw new Error('The native journal did not retain the adoption');
    expect(eventRecord.outcome).toBe('succeeded'); expect(eventRecord.revision).toBe('2');
    const eventCommand = eventRecord.intent.commandId; expect(eventCommand).toMatch(/^owner-adopt-[0-9a-f]{32}$/);
    await expect(dialog.locator('.proposal-history')).toContainText(eventCommand);
    expect(submitted).toHaveLength(1); expect(submitted[0]!['meta']).toBeUndefined();
    expect(submitted[0]!['params']['commandId']).toBeUndefined();
    const adoptedEvent = (await rpc<ReviewEnvelope>('automation.review', { kind: 'event', id }, true)).result;
    expect(adoptedEvent.record?.['enabled']).toBe(false); expect(adoptedEvent.provenance?.authority.kind).toBe('owner');
    expect(adoptedEvent.dependencies[0]!.provenance?.authority.kind).toBe('delegated');
    await dialog.locator('.base-dialog__footer').getByRole('button', { name: words.close, exact: true }).click();
    await expect(editorName).toHaveValue(`Unsaved owner draft ${locale}`);
    await noBrowserJournal(page, locale, [eventCommand]);
    await page.reload();
    await page.locator('.automation-library tbody tr').filter({ hasText: `Moderator event ${locale}` }).waitFor({ state: 'attached' });
    const restoredSheet = page.locator('.automations-page__sheet');
    if (await restoredSheet.isVisible()) await restoredSheet.getByRole('button', { name: locale === 'es' ? 'Atrás' : 'Back', exact: true }).click();
    await page.getByRole('button', { name: words.history, exact: true }).filter({ visible: true }).first().click();
    dialog = reviewDialog(page);
    await expect(dialog.locator('.proposal-history')).toContainText(eventCommand);
    expect(submitted).toHaveLength(1);
    await noHorizontalOverflow(dialog);
    await page.screenshot({ path: `${shots}/owner-proposal-history-${locale}-${locale === 'es' ? '390' : 'desktop'}.png` });
    await dialog.locator('.base-dialog__footer').getByRole('button', { name: words.close, exact: true }).click();
    await page.getByRole('tab', { name: words.actions }).click();
    await page.locator('.automation-library tbody tr').filter({ hasText: `Moderator action ${locale}` }).click();
    dialog = await openReview(page, words.open);
    await expect(dialog.locator('.reviewed-definition').first()).toContainText(words.delegated);
    await expect(dialog.locator('.proposal-dependencies details')).toHaveCount(0);
    await dialog.locator('[data-test="adoption-acknowledge"]').check();
    await dialog.locator('[data-test="adoption-submit"]').click();
    await expect.poll(async () => (await rpc<AdoptionState>('automation.adoption.get')).recent.filter((entry) => entry.intent.id === id)).toHaveLength(2);
    const action = (await rpc<ReviewEnvelope>('automation.review', { kind: 'action', id }, true)).result;
    expect(action.record?.['enabled']).toBe(false); expect(action.provenance?.authority.kind).toBe('owner');
    expect(submitted).toHaveLength(2);
    expect(methods.some((method) => ['automation.enable', 'automation.fire', 'automation.test', 'live.connect', 'points.adjust'].includes(method))).toBe(false);
    expect((await rpc('automation.runs'))['runs']).toEqual([]);
    await noBrowserJournal(page, locale, [eventCommand]);
    expect(failures).toEqual([]);
  });
}
