/** Real owner UI driver. Private invitation handoff uses inherited pipes only. */
import { chromium } from '@playwright/test';
import { createInterface } from 'node:readline';

const reader = createInterface({ input: process.stdin });
const input = reader[Symbol.asyncIterator]();
async function read(): Promise<Record<string, unknown>> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const item = await Promise.race([
      input.next(),
      new Promise<never>((_, reject) => { timer = setTimeout(() => reject(new Error('Owner test protocol timed out')), 18000); }),
    ]);
    if (item.done) throw new Error('Owner test protocol ended early');
    return JSON.parse(item.value) as Record<string, unknown>;
  } finally { clearTimeout(timer); }
}
function emit(value: unknown): void { process.stdout.write(`${JSON.stringify(value)}\n`); }
function check(value: unknown, message: string): asserts value { if (!value) throw new Error(message); }
const browser = await chromium.launch();
try {
  const context = await browser.newContext({ locale: 'en-US', viewport: { width: 1200, height: 900 } });
  const page = await context.newPage(); page.setDefaultTimeout(20000);
  const failures: string[] = [];
  page.on('pageerror', () => failures.push('Owner page error'));
  const setup = await read();
  const ownerUrl = String(setup['ownerUrl']); const owner = new URL(ownerUrl);
  page.on('request', (request) => {
    const url = new URL(request.url());
    if (!['http:', 'ws:', 'blob:', 'data:'].includes(url.protocol) || !['blob:', 'data:'].includes(url.protocol) && url.host !== owner.host) failures.push('Unexpected external owner page request');
  });
  await page.goto(ownerUrl);
  await page.getByLabel('Pairing code', { exact: true }).fill('948231');
  await page.getByRole('button', { name: 'Connect', exact: true }).click();
  await page.getByRole('link', { name: 'Settings', exact: true }).waitFor();
  await page.goto(`${ownerUrl}/#/settings?section=collaboration`);
  await page.getByRole('checkbox', { name: 'Allow remote controllers', exact: true }).check();
  await page.getByLabel('Listen address and port', { exact: true }).fill(String(setup['bind']));
  await page.getByLabel('Public HTTPS address', { exact: true }).fill(String(setup['origin']));
  await page.getByLabel('Certificate chain file', { exact: true }).fill(String(setup['certificate']));
  await page.getByLabel('Private key file', { exact: true }).fill(String(setup['key']));
  await page.getByRole('button', { name: 'Apply configuration', exact: true }).click();
  const invitations = page.getByRole('group', { name: 'Invite a controller', exact: true });
  await invitations.getByRole('button', { name: 'Workflow reviewer', exact: true }).click();
  await invitations.getByText('Resource restrictions', { exact: true }).click();
  await invitations.getByRole('combobox', { name: /^Workflows/ }).selectOption('all');
  await invitations.getByRole('checkbox', { name: 'Read automation status', exact: true }).check();
  await invitations.getByRole('checkbox', { name: 'Disable complete events (requires read access)', exact: true }).check();
  await invitations.getByRole('checkbox', { name: 'Read own command outcomes', exact: true }).check();
  await invitations.getByRole('combobox', { name: /^Automations/ }).selectOption('selected');
  await invitations.getByRole('textbox', { name: /^Automations — Exact IDs/ }).fill('event:welcome\naction:welcome');

  for (let index = 0; index < 2; index += 1) {
    await invitations.getByRole('button', { name: 'Create private invitation', exact: true }).click();
    const link = page.locator('#collaboration-link'); await link.waitFor();
    const value = new URL(await link.inputValue());
    check(value.origin === setup['origin'], 'Invitation belongs to another origin');
    const fragment = new URLSearchParams(value.hash.slice(1));
    const inviteId = fragment.get('inviteId'); const secret = fragment.get('secret');
    check(inviteId && secret && /^[a-f0-9]{64}$/.test(secret), 'Invalid private invitation');
    emit({ event: 'invitation', inviteId, secret });
    await invitations.getByRole('button', { name: 'Hide private link', exact: true }).click();
    const approval = await read(); check(approval['type'] === 'approve', 'Expected approval handoff');
    const requestId = String(approval['requestId']);
    const request = page.locator('article').filter({ hasText: requestId });
    await request.getByRole('button', { name: 'Approve device', exact: true }).click();
    const dialog = page.getByRole('dialog');
    check((await dialog.textContent())?.includes(requestId), 'Approval request identity mismatch');
    await dialog.getByRole('button', { name: 'Apply', exact: true }).click(); await dialog.waitFor({ state: 'hidden' });
    emit({ event: 'approved' });
    const next = await read(); check(next['type'] === 'connected', 'Expected connected session');
  }
  const revocation = await read(); check(revocation['type'] === 'revoke', 'Expected session revocation');
  await page.getByRole('button', { name: 'Device sessions', exact: true }).click();
  const session = page.locator('article').filter({ hasText: String(revocation['sessionId']) });
  await session.getByText('Connected (1)', { exact: true }).waitFor();
  await session.getByRole('button', { name: 'Revoke', exact: true }).click();
  const dialog = page.getByRole('dialog'); check((await dialog.textContent())?.includes('Only this device session'), 'Wrong revocation scope');
  await dialog.getByRole('button', { name: 'Apply', exact: true }).click(); await dialog.waitFor({ state: 'hidden' });
  await session.getByText('Revoked', { exact: true }).waitFor(); emit({ event: 'revoked' });
  const disable = await read(); check(disable['type'] === 'disable', 'Expected disable command');
  await page.getByRole('button', { name: 'Disable remote access', exact: true }).click();
  check((await dialog.textContent())?.includes('The local LIVE will continue'), 'Disable scope is missing');
  await dialog.getByRole('button', { name: 'Apply', exact: true }).click(); await dialog.waitFor({ state: 'hidden' });
  await page.getByText('Remote listener is stopped', { exact: true }).waitFor();
  check(!failures.length, failures[0] ?? 'Owner browser failed');
  emit({ event: 'disabled' });
} finally { reader.close(); process.stdin.pause(); await browser.close(); }
