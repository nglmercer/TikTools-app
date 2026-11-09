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
  const cookies = await context.cookies();
  if (!cookies.some((cookie) => cookie.name === '__Host-tiktools-remote' && cookie.httpOnly && cookie.secure)) throw new Error('session cookie flags missing');
  if (await page.evaluate(() => Object.keys(localStorage).some((key) => /token|credential|secret/i.test(key)))) throw new Error('credential-like browser storage');
  if (leaked) throw new Error('invitation secret appeared in a request');
  mark('ready');
  await page.getByRole('button', { name: /Host updated/ }).waitFor({ timeout: 20_000 });
  mark('updated');
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
  await replacement.getByRole('button', { name: /Host updated/ }).waitFor({ timeout: 20_000 });
  if (leaked) throw new Error('invitation secret appeared in a request');
  mark('replacement-ready');
  await replacement.getByRole('button', { name: /Host updated/ }).waitFor({ state: 'hidden', timeout: 20_000 });
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
