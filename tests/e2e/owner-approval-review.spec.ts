/** Packaged Vue → native owner approval coordinator → IPC → one real Core.
 * Invoked by Rust owner_approval_browser; no fake host or response interception. */
import { expect, test, type BrowserContext, type Locator, type Page } from '@playwright/test';
import { execFile } from 'node:child_process';
import { mkdir } from 'node:fs/promises';
import { promisify } from 'node:util';
import type { ApprovalState } from '../../src/web/features/review-automation-approval/model/native-contracts.ts';
import type { CapturedApprovalReview } from '../../src/web/features/review-automation-approval/model/contracts.ts';
import english from '../../src/web/shared/i18n/locales/automation-approval.en.ts';
import spanish from '../../src/web/shared/i18n/locales/automation-approval.es.ts';
import { openHistoryEntry } from './fixtures/automation-history.ts';

const execute = promisify(execFile);
const ownerUrl = process.env['TIKTOOLS_OWNER_APPROVAL_URL'];
const home = process.env['TIKTOOLS_OWNER_APPROVAL_HOME'];
const binary = process.env['TIKTOOLS_OWNER_APPROVAL_BINARY'];
const shots = '/home/meme/.cache/tiktools-test-runs';
type Json = Record<string, unknown>;
interface SentRequest { method: string; params: Json; meta?: unknown }
interface AutomationRun { id: string; actionId: string; eventId: string; status: string; logs: string[]; test: boolean }
type Review = Pick<CapturedApprovalReview, 'source' | 'manifest' | 'subjects' | 'approval' | 'eligibility' | 'hostId' | 'hostInstanceId'> & { kind: 'action' | 'event'; id: string };
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

function dialog(page: Page): Locator { return page.getByRole('dialog').filter({ has: page.getByTestId('approval-journal') }); }
async function openReview(page: Page): Promise<Locator> {
  await openHistoryEntry(page, 'approval-open', 'review-approval');
  const view = dialog(page);
  await expect(view.getByTestId('approval-source').locator('pre')).toBeVisible();
  return view;
}
async function noBrowserJournal(page: Page, locale: string, commands: string[]): Promise<void> {
  const storage = await page.evaluate(() => JSON.stringify({ local: Object.entries(localStorage), session: Object.entries(sessionStorage) }));
  for (const forbidden of ['owner-approval-', `approval-source-${locale}`, `approval-script-${locale}`, `approval-script-source-${locale}`, `script-runtime-${locale}`, `script-console-${locale}`, `Unsaved approval draft ${locale}`, ...commands]) {
    expect(storage).not.toContain(forbidden);
  }
}
async function noOverflow(view: Locator): Promise<void> {
  expect(await view.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
  for (const label of await view.locator('button:visible .base-button__label').all()) {
    expect(await label.evaluate((node) => node.scrollWidth <= node.clientWidth + 1)).toBe(true);
  }
}
async function balance(): Promise<number> {
  return (await rpc<{ viewer: { points: number } }>('points.viewer.get', { uniqueId: 'alice' })).viewer.points;
}
async function fire(): Promise<void> {
  await rpc('automation.fire', { trigger: 'tiktok.chat', event: { user: { uniqueId: 'alice' }, data: { comment: 'Explicit isolated approval fixture' } } });
}
async function scriptRuns(actionId: string): Promise<AutomationRun[]> {
  return (await rpc<{ runs: AutomationRun[] }>('automation.runs')).runs.filter((run) => run.actionId === actionId && !run.test);
}
async function completedSubmission(request: SentRequest): Promise<ApprovalState> {
  let settled: ApprovalState | undefined;
  // Historical successes belong to earlier commands. Wait for the exact
  // submitted transition to become a native terminal receipt, not for text
  // already visible in the journal or an older operation of the same kind.
  await expect.poll(async () => {
    settled = await rpc<ApprovalState>('automation.approval.get');
    const record = settled.recent.find((entry) => entry.intent.kind === request.params['kind'] &&
      entry.intent.id === request.params['id'] && entry.intent.operation === request.params['operation'] &&
      entry.intent.expectedRevision === request.params['expectedRevision'] &&
      entry.intent.hostInstanceId === request.params['hostInstanceId']);
    return { outcome: record?.outcome ?? null, pending: settled.pending !== null };
  }).toEqual({ outcome: 'succeeded', pending: false });
  return settled!;
}
async function closeReview(page: Page, view: Locator, locale: 'en' | 'es'): Promise<void> {
  await view.locator('.base-dialog__footer').getByRole('button', { name: locale === 'es' ? 'Cerrar' : 'Close', exact: true }).click();
  const sheet = page.locator('.automations-page__sheet');
  if (await sheet.isVisible()) await sheet.getByRole('button', { name: locale === 'es' ? 'Atrás' : 'Back', exact: true }).click();
}

for (const locale of ['en', 'es'] as const) {
  test(`real owner approval, explicit plan, stale review and native recovery (${locale})`, async ({ page }) => {
    test.skip(!ownerUrl || !home || !binary, 'Requires Rust owner_approval_browser fixture');
    const words = locale === 'en' ? english : spanish;
    const id = `proposal-${locale}`; const alternative = `disabled-${locale}`; const ownerAction = `owner-${locale}`;
    const sent: SentRequest[] = []; const failures: string[] = []; const methods: string[] = [];
    const origin = new URL(ownerUrl!).host;
    page.on('pageerror', (error) => failures.push(error.message));
    page.on('request', (request) => {
      const url = new URL(request.url());
      if (!['blob:', 'data:'].includes(url.protocol) && url.host !== origin) failures.push('Unexpected external request');
    });
    page.on('websocket', (socket) => socket.on('framesent', (frame) => {
      try {
        const value = JSON.parse(String(frame.payload)) as SentRequest;
        if (typeof value.method === 'string') methods.push(value.method);
        if (value.method === 'automation.approval.submit') sent.push(value);
      } catch { /* Transport pings are not commands. */ }
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
    await page.locator('.event-editor:visible input[type="text"]').first().fill(`Unsaved approval draft ${locale}`);
    let view = await openReview(page);
    const original = await rpc<Review>('automation.approval.review', { kind: 'event', id });
    expect(original.eligibility.eligible).toBe(true);
    expect(original.source.provenance?.authority.kind).toBe('delegated');
    expect(JSON.parse(await view.getByTestId('approval-source').locator('pre').innerText())).toEqual(original.source.record);
    expect(original.source.dependencies.map((d) => d.id)).toEqual([id, alternative, ownerAction]);
    await expect(view.getByTestId('approval-effect')).toHaveCount(3);
    await expect(view.getByTestId('approval-plan-include-' + ownerAction)).toHaveCount(0);
    await expect(view.getByTestId('approval-submit')).toBeDisabled();
    await view.getByTestId('approval-subject').locator('summary').click();
    await expect(view.getByTestId('approval-subject')).toContainText('points.adjust');
    await expect(view.getByTestId('approval-subject')).toContainText('alice');
    expect(original.subjects[0]!.grant.maxPointsDelta).toBe(2);
    expect(original.manifest?.subjects[0]!.configurationRevision).toBe(original.subjects[0]!.configurationRevision);
    for (const dependency of original.source.dependencies) {
      const source = view.getByTestId('approval-dependency-source-' + dependency.id);
      await source.locator('xpath=ancestor::details[1]').locator(':scope > summary').click();
      expect(JSON.parse(await source.locator('pre').innerText())).toEqual(dependency.record);
    }
    await view.getByTestId('approval-plan-include-' + id).check();
    await expect(view.getByTestId('approval-plan-enabled-' + id)).toBeChecked();
    await expect(view.getByTestId('approval-plan-include-' + alternative)).not.toBeChecked();
    await view.getByTestId('approval-acknowledge').check();
    await expect(view.getByTestId('approval-submit')).toBeEnabled();
    await noOverflow(view);
    await mkdir(shots, { recursive: true });
    await view.locator('.base-dialog__body').evaluate((body) => { body.scrollTop = 0; });
    await view.screenshot({ path: `${shots}/owner-approval-${locale}-review.png` });
    await view.getByTestId('approval-submit').click();
    await expect.poll(() => sent.length).toBe(1);
    const request = sent[0]!;
    const state = await completedSubmission(request);
    const activation = state.recent.find((entry) => entry.intent.id === id && entry.intent.operation === 'activate')!;
    expect(activation.outcome).toBe('succeeded');
    expect(state.pending).toBeNull();
    await expect(view.getByTestId('approval-journal').getByRole('listitem').filter({ hasText: activation.intent.commandId })).toContainText(words.succeeded);
    expect(request.meta).toBeUndefined();
    expect(Object.keys(request.params).sort()).toEqual(['operation', 'kind', 'id', 'expectedRevision', 'hostId', 'hostInstanceId', 'reviewedVersions', 'manifestDigest', 'enablePlan', 'acknowledgeEffects'].sort());
    expect(request.params['enablePlan']).toEqual([{ kind: 'event', id, enabled: true }, { kind: 'action', id, enabled: true }]);
    const active = await rpc<Review>('automation.approval.review', { kind: 'event', id });
    expect(active.approval.current).toBe(true);
    expect(active.approval.manifestDigest).toBe(activation.receipt!.manifestDigest);
    expect(active.approval.manifestDigest).not.toBe(original.manifest!.manifestDigest);
    expect(active.source.provenance?.authority.kind).toBe('delegated');
    expect(active.source.dependencies[0]!.provenance?.authority.kind).toBe('delegated');
    expect(active.source.dependencies.map((d) => d.record?.['enabled'])).toEqual([true, false, false]);
    expect(active.source.dependencies[2]!.provenance?.authority.kind).toBe('owner');
    const before = await balance();
    await fire();
    expect(await balance()).toBe(before + 2);
    await noBrowserJournal(page, locale, [activation.intent.commandId]);

    await page.reload();
    const restoredSheet = page.locator('.automations-page__sheet');
    if (locale === 'es') await expect(restoredSheet).toBeVisible();
    if (await restoredSheet.isVisible()) {
      await restoredSheet.getByRole('button', { name: locale === 'es' ? 'Atrás' : 'Back', exact: true }).click();
    }
    await openHistoryEntry(page, 'approval-history', 'approval');
    view = dialog(page);
    await expect(view.getByTestId('approval-journal')).toContainText(activation.intent.commandId);
    await expect(view.getByTestId('approval-source')).toHaveCount(0);
    expect(sent).toHaveLength(1);
    await noBrowserJournal(page, locale, [activation.intent.commandId]);
    await view.locator('.base-dialog__footer').getByRole('button', { name: locale === 'es' ? 'Cerrar' : 'Close', exact: true }).click();
    await page.locator('.automation-library tbody tr').filter({ hasText: `Moderator event ${locale}` }).click();
    view = await openReview(page);
    await view.getByTestId('approval-acknowledge').check();
    await expect(view.getByTestId('approval-submit')).toBeEnabled();
    const changed = { ...active.source.dependencies[1]!.record!, config: { message: `changed approval-source-${locale}` } };
    await rpc('automation.update', { kind: 'action', id: alternative, record: changed });
    await expect(view.getByTestId('approval-submit')).toBeDisabled();
    await expect(view).toContainText(words.changed);
    expect(sent).toHaveLength(1);
    if (locale === 'es') {
      // The real host must keep revocation available even when the current
      // author/manifest can no longer be resolved. All proposals were already
      // seeded by this isolated author before the browser started.
      await rpc('collaboration.sessions.revoke', { id: active.subjects[0]!.sessionId });
    }
    await view.getByRole('button', { name: words.read, exact: true }).click();
    await expect(view.getByTestId('approval-status')).toContainText(words.invalidated);
    if (locale === 'es') {
      await expect(view.getByTestId('approval-status')).toContainText(words.subjectsUnavailable);
      await expect(view.getByTestId('approval-acknowledge')).toBeDisabled();
    }
    await view.getByTestId('approval-revoke-acknowledge').check();
    await view.getByTestId('approval-revoke').click();
    await expect.poll(() => sent.length).toBe(2);
    const revocationState = await completedSubmission(sent[1]!);
    const revocation = revocationState.recent.find((entry) => entry.intent.id === id && entry.intent.operation === 'revoke')!;
    expect(revocation.outcome).toBe('succeeded');
    await expect(view.getByTestId('approval-journal').getByRole('listitem').filter({ hasText: revocation.intent.commandId })).toContainText(words.succeeded);
    const revoked = await rpc<Review>('automation.approval.review', { kind: 'event', id });
    expect(revoked.approval.state).toBe('revoked');
    expect(revoked.approval.current).toBe(false);
    expect(sent).toHaveLength(2);
    expect(sent[1]!.params['operation']).toBe('revoke');
    expect(Object.keys(sent[1]!.params).sort()).toEqual(['operation', 'kind', 'id', 'expectedRevision', 'hostId', 'hostInstanceId', 'acknowledgeRevocation'].sort());
    const after = await balance();
    await fire();
    expect(await balance()).toBe(after);
    await noBrowserJournal(page, locale, (await rpc<ApprovalState>('automation.approval.get')).recent.map((entry) => entry.intent.commandId));
    await noOverflow(view);
    await view.screenshot({ path: `${shots}/owner-approval-${locale}-revoked.png` });

    // A second, independently paired author keeps this positive script case
    // available after the Spanish original author has been revoked above.
    // Preserve every original random-points case and its native command IDs.
    const scriptEvent = `script-event-${locale}`;
    const scriptAction = `script-action-${locale}`;
    await closeReview(page, view, locale);
    await page.locator('.automation-library tbody tr').filter({ hasText: `Moderator script event ${locale}` }).click();
    view = await openReview(page);
    const scriptOriginal = await rpc<Review>('automation.approval.review', { kind: 'event', id: scriptEvent });
    expect(scriptOriginal.eligibility.eligible).toBe(true);
    expect(scriptOriginal.source.record?.['enabled']).toBe(false);
    expect(scriptOriginal.source.provenance?.authority.kind).toBe('delegated');
    expect(scriptOriginal.source.dependencies.map((dependency) => dependency.id)).toEqual([scriptAction]);
    expect(scriptOriginal.source.dependencies[0]!.record?.['enabled']).toBe(false);
    expect(scriptOriginal.source.dependencies[0]!.record?.['typeId']).toBe('core.code');
    expect(scriptOriginal.source.dependencies[0]!.provenance?.authority.kind).toBe('delegated');
    expect(scriptOriginal.subjects[0]!.principalId).not.toBe(active.subjects[0]!.principalId);
    expect(scriptOriginal.subjects[0]!.grant.capabilities).toContain('scripts.execute');
    expect(scriptOriginal.subjects[0]!.grant.capabilities).toContain('live.read');
    expect(scriptOriginal.subjects[0]!.grant.capabilities).not.toContain('points.adjust');
    expect(scriptOriginal.manifest!.effects[0]!.kind).toBe('script');
    expect(scriptOriginal.manifest!.effects[0]!.requiredCapabilities).toEqual(expect.arrayContaining(['scripts.execute', 'live.read']));
    expect(JSON.parse(await view.getByTestId('approval-source').locator('pre').innerText())).toEqual(scriptOriginal.source.record);
    await expect(view.getByTestId('approval-effect')).toHaveCount(1);
    await expect(view.getByTestId('approval-script-policy')).toBeVisible();
    await expect(view.getByTestId('approval-script-policy')).toHaveText(words.scriptPolicy);
    const scriptSource = view.getByTestId('approval-dependency-source-' + scriptAction);
    await scriptSource.locator('xpath=ancestor::details[1]').locator(':scope > summary').click();
    expect(JSON.parse(await scriptSource.locator('pre').innerText())).toEqual(scriptOriginal.source.dependencies[0]!.record);
    expect((scriptOriginal.source.dependencies[0]!.record?.['config'] as Json)['source']).toContain(`script-runtime-${locale}`);
    await expect(view.getByTestId('approval-submit')).toBeDisabled();
    await expect(view.getByTestId('approval-plan-include-' + scriptAction)).not.toBeChecked();
    await view.getByTestId('approval-plan-include-' + scriptAction).check();
    await expect(view.getByTestId('approval-plan-enabled-' + scriptAction)).toBeChecked();
    await view.getByTestId('approval-acknowledge').check();
    await expect(view.getByTestId('approval-submit')).toBeEnabled();
    await noOverflow(view);
    await view.locator('.base-dialog__body').evaluate((body) => { body.scrollTop = 0; });
    await view.screenshot({ path: `${shots}/owner-approval-script-${locale}-review.png` });
    await view.getByTestId('approval-submit').click();
    await expect.poll(() => sent.length).toBe(3);
    const scriptState = await completedSubmission(sent[2]!);
    const scriptActivation = scriptState.recent.find((entry) => entry.intent.id === scriptEvent && entry.intent.operation === 'activate')!;
    expect(scriptActivation.outcome).toBe('succeeded');
    expect(scriptState.pending).toBeNull();
    await expect(view.getByTestId('approval-journal').getByRole('listitem').filter({ hasText: scriptActivation.intent.commandId })).toContainText(words.succeeded);
    expect(sent).toHaveLength(3);
    expect(sent[2]!.meta).toBeUndefined();
    expect(Object.keys(sent[2]!.params).sort()).toEqual(Object.keys(request.params).sort());
    expect(sent[2]!.params['enablePlan']).toEqual([{ kind: 'event', id: scriptEvent, enabled: true }, { kind: 'action', id: scriptAction, enabled: true }]);
    const scriptActive = await rpc<Review>('automation.approval.review', { kind: 'event', id: scriptEvent });
    expect(scriptActive.approval.current).toBe(true);
    expect(scriptActive.approval.manifestDigest).toBe(scriptActivation.receipt!.manifestDigest);
    expect(scriptActive.approval.manifestDigest).not.toBe(scriptOriginal.manifest!.manifestDigest);
    expect(scriptActive.source.dependencies[0]!.record?.['enabled']).toBe(true);
    expect(scriptActive.source.dependencies[0]!.provenance?.authority.kind).toBe('delegated');
    expect(scriptActive.source.dependencies[0]!.record?.['config']).toEqual(scriptOriginal.source.dependencies[0]!.record?.['config']);
    const beforeScriptBalance = await balance();
    expect(await scriptRuns(scriptAction)).toHaveLength(0);
    await fire();
    await expect.poll(async () => (await scriptRuns(scriptAction)).length).toBe(1);
    const scriptRun = (await scriptRuns(scriptAction))[0]!;
    expect(scriptRun.status).toBe('ok');
    expect(scriptRun.eventId).toBe(scriptEvent);
    expect(scriptRun.logs).toContain(`script-runtime-${locale}:alice`);
    expect(scriptRun.logs).toContain(`script-console-${locale}`);
    expect(await balance()).toBe(beforeScriptBalance);
    await noBrowserJournal(page, locale, scriptState.recent.map((entry) => entry.intent.commandId));

    await page.reload();
    const scriptSheet = page.locator('.automations-page__sheet');
    if (locale === 'es') await expect(scriptSheet).toBeVisible();
    if (await scriptSheet.isVisible()) await scriptSheet.getByRole('button', { name: locale === 'es' ? 'Atrás' : 'Back', exact: true }).click();
    await openHistoryEntry(page, 'approval-history', 'approval');
    view = dialog(page);
    await expect(view.getByTestId('approval-journal')).toContainText(scriptActivation.intent.commandId);
    await expect(view.getByTestId('approval-journal')).toContainText(activation.intent.commandId);
    await expect(view.getByTestId('approval-source')).toHaveCount(0);
    expect(sent).toHaveLength(3);
    await noBrowserJournal(page, locale, scriptState.recent.map((entry) => entry.intent.commandId));
    await closeReview(page, view, locale);
    await page.locator('.automation-library tbody tr').filter({ hasText: `Moderator script event ${locale}` }).click();
    view = await openReview(page);
    await expect(view.getByTestId('approval-status')).toContainText(words.active);
    await view.getByTestId('approval-revoke-acknowledge').check();
    await view.getByTestId('approval-revoke').click();
    await expect.poll(() => sent.length).toBe(4);
    const scriptRevocationState = await completedSubmission(sent[3]!);
    const scriptRevocation = scriptRevocationState.recent.find((entry) => entry.intent.id === scriptEvent && entry.intent.operation === 'revoke')!;
    expect(scriptRevocation.outcome).toBe('succeeded');
    await expect(view.getByTestId('approval-journal').getByRole('listitem').filter({ hasText: scriptRevocation.intent.commandId })).toContainText(words.succeeded);
    const scriptRevoked = await rpc<Review>('automation.approval.review', { kind: 'event', id: scriptEvent });
    expect(scriptRevoked.approval.state).toBe('revoked');
    expect(scriptRevoked.approval.current).toBe(false);
    expect(sent).toHaveLength(4);
    expect(sent[3]!.params['operation']).toBe('revoke');
    expect(Object.keys(sent[3]!.params).sort()).toEqual(Object.keys(sent[1]!.params).sort());
    await fire();
    expect(await scriptRuns(scriptAction)).toHaveLength(1);
    expect(await balance()).toBe(beforeScriptBalance);
    await noBrowserJournal(page, locale, (await rpc<ApprovalState>('automation.approval.get')).recent.map((entry) => entry.intent.commandId));
    await noOverflow(view);
    await view.screenshot({ path: `${shots}/owner-approval-script-${locale}-revoked.png` });
    expect(methods).not.toContain('automation.approval.activate');
    expect(methods).not.toContain('automation.approval.revoke');
    expect(methods).not.toContain('automation.adoption.submit');
    expect(failures).toEqual([]);
  });
}
