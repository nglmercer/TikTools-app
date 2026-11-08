/** Actual packaged owner editors → local browser adapter → IPC host. */
import { chromium, expect, type Page } from '@playwright/test';
import { createInterface } from 'node:readline';

const reader = createInterface({ input: process.stdin });
const input = reader[Symbol.asyncIterator]();
async function read(): Promise<Record<string, unknown>> {
  let timer: ReturnType<typeof setTimeout> | undefined;
  try {
    const item = await Promise.race([input.next(), new Promise<never>((_, reject) => {
      timer = setTimeout(() => reject(new Error('Owner editor protocol timed out')), 20_000);
    })]);
    if (item.done) throw new Error('Owner editor protocol ended early');
    return JSON.parse(item.value) as Record<string, unknown>;
  } finally { clearTimeout(timer); }
}
function emit(event: string): void { process.stdout.write(`${JSON.stringify({ event })}\n`); }
const browser = await chromium.launch();
try {
  const setup = await read();
  const url = String(setup['ownerUrl']); const origin = new URL(url).host;
  const context = await browser.newContext({ locale: 'en-US', viewport: { width: 1280, height: 900 } });
  const failures: string[] = [];
  context.on('page', (page) => {
    page.setDefaultTimeout(20_000);
    page.on('pageerror', () => failures.push('Owner editor page error'));
    page.on('request', (request) => {
      const parsed = new URL(request.url());
      if (!['blob:', 'data:'].includes(parsed.protocol) && parsed.host !== origin) failures.push('Unexpected external request');
    });
  });
  const first = await context.newPage();
  await first.goto(url);
  await first.getByLabel('Pairing code', { exact: true }).fill('948231');
  await first.getByRole('button', { name: 'Connect', exact: true }).click();
  await first.getByRole('link', { name: 'Settings', exact: true }).waitFor();
  await first.goto(`${url}/#/automations`);
  const second = await context.newPage(); await second.goto(`${url}/#/automations`);
  const name = (page: Page) => page.locator('.event-editor input[type="text"]').first();
  const save = (page: Page) => page.getByRole('button', { name: 'Save', exact: true });
  const saved = async (page: Page) => {
    await expect(page.locator('.event-editor').getByText('Unsaved changes', { exact: true })).toHaveCount(0);
    await expect(save(page)).toBeDisabled();
  };
  for (const page of [first, second]) await page.locator('.automation-library tbody tr').filter({ hasText: 'Shared event' }).click();
  await name(first).fill('First owner draft'); await name(second).fill('Second owner draft');
  emit('editors-open'); await read();
  await save(first).click(); await saved(first); emit('first-saved'); await read();
  await expect(name(second)).toHaveValue('Second owner draft');
  await save(second).click();
  await expect(second.locator('.automation-edit-notice')).toContainText('The change was not accepted');
  await expect(name(second)).toHaveValue('Second owner draft');
  await second.getByRole('button', { name: 'Compare with host', exact: true }).click();
  const dialog = second.getByRole('dialog').filter({ has: second.locator('.automation-review') });
  await expect(dialog).toContainText('Shared event'); await expect(dialog).toContainText('First owner draft'); await expect(dialog).toContainText('Second owner draft');
  await dialog.getByRole('button', { name: 'Use this version as the draft base', exact: true }).click();
  emit('reviewed'); await read();
  await save(second).click(); await saved(second); emit('second-saved'); await read();
  await name(second).fill('Recovered after deletion');
  await save(second).click();
  await expect(second.locator('.automation-edit-notice')).toContainText('The change was not accepted');
  await second.getByRole('button', { name: 'Compare with host', exact: true }).click();
  await expect(dialog).toContainText('The record is absent on the host.');
  await dialog.getByRole('button', { name: 'Prepare recreation with this draft', exact: true }).click();
  emit('recreation-prepared'); await read();
  await save(second).click(); await saved(second);
  await expect(name(second)).toHaveValue('Recovered after deletion');
  if (failures.length) throw new Error(failures[0]);
  emit('recreated');
} finally { reader.close(); process.stdin.pause(); await browser.close(); }
