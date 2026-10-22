import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

const source = readFileSync(resolve(process.cwd(), 'crates/tiktools-cli/src/web.rs'), 'utf8');
const page = source.match(/const PAIR_PAGE: &str = r#"([\s\S]*?)"#;/)![1]!;
const script = source.match(/const PAIR_SCRIPT: &str = r#"([\s\S]*?)"#;/)![1]!;
let request: ReturnType<typeof vi.fn>;

function mount() {
  const body = page.split('<body>')[1]!.split('</body>')[0]!;
  document.body.innerHTML = body.replace(/<script\b[^>]*>[\s\S]*?<\/script>/g, '');
  new Function(script)();
  return {
    renew: document.getElementById('renew') as HTMLButtonElement,
    input: document.getElementById('code') as HTMLInputElement,
    result: document.getElementById('result')!,
    form: document.getElementById('pair') as HTMLFormElement,
  };
}
function response(status: number, body: unknown) {
  return new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
}
beforeEach(() => {
  vi.useFakeTimers();
  request = vi.fn(); vi.stubGlobal('fetch', request);
});
afterEach(() => {
  vi.clearAllTimers(); vi.useRealTimers(); vi.unstubAllGlobals();
  document.body.replaceChildren();
});

test('requesting a new code uses POST, clears old input and waits before another renewal', async () => {
  request.mockResolvedValue(response(200, { ok: true, expiresInSeconds: 300, retryAfterSeconds: 30 }));
  const ui = mount(); ui.input.value = '012345'; ui.renew.click();
  expect(ui.renew.disabled).toBe(true);
  expect(request).toHaveBeenCalledExactlyOnceWith('/__local/pair/renew', { method: 'POST' });
  expect(ui.input.value).toBe('');
  await vi.waitFor(() => expect(ui.result.textContent).toContain('New code printed in Termux'));
  expect(ui.result.dataset.kind).toBe('success');
  ui.renew.click(); expect(request).toHaveBeenCalledTimes(1);
  await vi.advanceTimersByTimeAsync(30_000); expect(ui.renew.disabled).toBe(false);
});

test('cooldown and lockout responses remain distinct and never imply a code was issued', async () => {
  request.mockResolvedValueOnce(response(429, { error: 'cooldown', retryAfterSeconds: 2 }));
  const ui = mount(); ui.renew.click();
  await vi.waitFor(() => expect(ui.result.textContent).toContain('wait 2 seconds'));
  expect(ui.renew.disabled).toBe(true);
  await vi.advanceTimersByTimeAsync(2000); expect(ui.renew.disabled).toBe(false);
  request.mockResolvedValueOnce(response(429, { error: 'pairing_locked' })); ui.renew.click();
  await vi.waitFor(() => expect(ui.result.textContent).toContain('Too many attempts'));
  await vi.advanceTimersByTimeAsync(60_000); expect(ui.renew.disabled).toBe(true);
  expect(ui.result.textContent).not.toContain('New code printed');
});

test('a network failure permits retry and an active session is not described as renewed', async () => {
  request.mockRejectedValueOnce(new Error('offline'));
  const ui = mount(); ui.renew.click();
  await vi.waitFor(() => expect(ui.result.textContent).toContain('Adapter unavailable'));
  expect(ui.renew.disabled).toBe(false);
  request.mockResolvedValueOnce(response(409, { error: 'already_paired' })); ui.renew.click();
  await vi.waitFor(() => expect(ui.result.textContent).toContain('Already paired'));
  expect(ui.renew.disabled).toBe(true);
});

test('sign-in still submits leading zeros as a string and clears the field', async () => {
  request.mockResolvedValue(response(401, {}));
  const ui = mount(); ui.input.value = '012345';
  ui.form.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
  expect(request).toHaveBeenCalledExactlyOnceWith('/__local/pair', {
    method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ code: '012345' }),
  });
  expect(ui.input.value).toBe('');
  await vi.waitFor(() => expect(ui.result.textContent).toContain('request a new code'));
});
