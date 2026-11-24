import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { supportedLocales } from '../../../src/web/shared/i18n/supported-locales.ts';
import { translatorFor, type MessageKey } from '../../../src/pairing/messages.ts';
import { mountPairing, resolveLocale, type PairingEnvironment } from '../../../src/pairing/pairing.ts';

const template = readFileSync(resolve(process.cwd(), 'src/pairing/pairing.html'), 'utf8');
const bodyMarkup = template
  .split('<body>')[1]!
  .split('</body>')[0]!
  .replace(/<script\b[^>]*>[\s\S]*?<\/script>/g, '');
const en = translatorFor('en');
const fr = translatorFor('fr');

const MESSAGE_KEYS: readonly MessageKey[] = [
  'documentTitle', 'title', 'intro', 'label', 'connect', 'renew', 'invalidCode',
  'adapterUnavailable', 'signingIn', 'requesting', 'newCodePrinted', 'waitSeconds',
  'lockedOut', 'alreadyPaired', 'renewFailed',
];

function response(status: number, body: unknown) {
  return new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
}

function mount(options: { hash?: string; languages?: string[]; initial?: () => Response } = {}) {
  document.body.innerHTML = bodyMarkup;
  const fetch = vi.fn<PairingEnvironment['fetch']>(async () => options.initial?.() ?? response(401, {}));
  const clearFragment = vi.fn();
  const goHome = vi.fn();
  mountPairing(document, {
    fetch,
    languages: options.languages ?? ['en-US'],
    hash: options.hash ?? '',
    clearFragment,
    goHome,
  });
  return {
    fetch,
    clearFragment,
    goHome,
    renew: document.getElementById('renew') as HTMLButtonElement,
    input: document.getElementById('code') as HTMLInputElement,
    result: document.getElementById('result')!,
    form: document.getElementById('pair') as HTMLFormElement,
  };
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.clearAllTimers();
  vi.useRealTimers();
  document.body.replaceChildren();
});

test('requesting a new code uses POST, clears the old input and waits before another renewal', async () => {
  const ui = mount();
  ui.fetch.mockResolvedValueOnce(response(200, { ok: true }));
  ui.input.value = '012345';
  ui.renew.click();
  expect(ui.renew.disabled).toBe(true);
  expect(ui.fetch).toHaveBeenCalledExactlyOnceWith('/__local/pair/renew', { method: 'POST' });
  expect(ui.input.value).toBe('');
  await vi.waitFor(() => expect(ui.result.textContent).toBe(en('newCodePrinted')));
  expect(ui.result.dataset.kind).toBe('success');
  ui.renew.click();
  expect(ui.fetch).toHaveBeenCalledTimes(1);
  await vi.advanceTimersByTimeAsync(30_000);
  expect(ui.renew.disabled).toBe(false);
});

test('cooldown and lockout responses remain distinct and never imply a code was issued', async () => {
  const ui = mount();
  ui.fetch.mockResolvedValueOnce(response(429, { error: 'cooldown', retryAfterSeconds: 2 }));
  ui.renew.click();
  await vi.waitFor(() => expect(ui.result.textContent).toBe(en('waitSeconds', { seconds: 2 })));
  expect(ui.renew.disabled).toBe(true);
  await vi.advanceTimersByTimeAsync(2000);
  expect(ui.renew.disabled).toBe(false);
  ui.fetch.mockResolvedValueOnce(response(429, { error: 'pairing_locked' }));
  ui.renew.click();
  await vi.waitFor(() => expect(ui.result.textContent).toBe(en('lockedOut')));
  await vi.advanceTimersByTimeAsync(60_000);
  expect(ui.renew.disabled).toBe(true);
  expect(ui.result.textContent).not.toBe(en('newCodePrinted'));
});

test('a network failure permits retry and an active session is not described as renewed', async () => {
  const ui = mount();
  ui.fetch.mockRejectedValueOnce(new Error('offline'));
  ui.renew.click();
  await vi.waitFor(() => expect(ui.result.textContent).toBe(en('adapterUnavailable')));
  expect(ui.renew.disabled).toBe(false);
  ui.fetch.mockResolvedValueOnce(response(409, { error: 'already_paired' }));
  ui.renew.click();
  await vi.waitFor(() => expect(ui.result.textContent).toBe(en('alreadyPaired')));
  expect(ui.renew.disabled).toBe(true);
});

test('sign-in submits leading zeros as a string, clears the field and reports a rejected code', async () => {
  const ui = mount();
  ui.input.value = '012345';
  ui.form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  expect(ui.fetch).toHaveBeenCalledExactlyOnceWith('/__local/pair', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ code: '012345' }),
  });
  expect(ui.input.value).toBe('');
  await vi.waitFor(() => expect(ui.result.textContent).toBe(en('invalidCode')));
});

test('a successful sign-in leaves the page for the adapter root', async () => {
  const ui = mount({ initial: () => response(200, { ok: true }) });
  ui.input.value = '654321';
  ui.form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  await vi.waitFor(() => expect(ui.goHome).toHaveBeenCalledOnce());
});

test('a one-click join link pairs once without typing and clears the fragment first', async () => {
  const ui = mount({ hash: '#pair=012345' });
  expect(ui.clearFragment).toHaveBeenCalledOnce();
  expect(ui.fetch).toHaveBeenCalledExactlyOnceWith('/__local/pair', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ code: '012345' }),
  });
  expect(ui.input.value).toBe('');
  await vi.waitFor(() => expect(ui.result.textContent).toBe(en('invalidCode')));
});

test('a malformed join fragment never auto-submits', () => {
  const ui = mount({ hash: '#pair=abcdef' });
  expect(ui.fetch).not.toHaveBeenCalled();
  expect(ui.clearFragment).not.toHaveBeenCalled();
});

test('the page localizes every visible string from the browser language', () => {
  const ui = mount({ languages: ['fr-FR', 'en'] });
  expect(document.documentElement.lang).toBe('fr');
  expect(document.title).toBe(fr('documentTitle'));
  expect(document.getElementById('connect')!.textContent).toBe(fr('connect'));
  expect(document.getElementById('renew')!.textContent).toBe(fr('renew'));
  expect(ui.renew.textContent).toBe(fr('renew'));
});

test('unsupported languages fall back to English and the jp alias resolves to Japanese', () => {
  expect(resolveLocale(['de-DE', 'it'])).toBe('en');
  expect(resolveLocale([])).toBe('en');
  expect(resolveLocale(['jp'])).toBe('ja');
  expect(resolveLocale(['zh-CN'])).toBe('zh');
});

test('every locale defines every message and the wait message keeps its placeholder', () => {
  for (const locale of supportedLocales) {
    const t = translatorFor(locale);
    for (const key of MESSAGE_KEYS) {
      expect(t(key), `${locale}.${key}`).not.toBe('');
      expect(t(key), `${locale}.${key}`).not.toBe(key);
    }
    expect(t('waitSeconds', { seconds: 7 })).toContain('7');
    expect(t('waitSeconds', { seconds: 7 })).not.toContain('{seconds}');
  }
});

test('the page template carries no literal user-visible text; all text comes from the catalog', () => {
  const visible = bodyMarkup
    .replace(/<style[\s\S]*?<\/style>/g, '')
    .replace(/<[^>]+>/g, ' ')
    .trim();
  expect(visible).toBe('');
});
