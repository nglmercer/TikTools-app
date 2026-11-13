import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';

const page = readFileSync(resolve(process.cwd(), 'crates/tiktools-cli/src/web/pairing.html'), 'utf8');
const script = readFileSync(resolve(process.cwd(), 'crates/tiktools-cli/src/web/pairing.js'), 'utf8');
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
  await vi.waitFor(() => expect(ui.result.textContent).toContain('New code printed in the terminal'));
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

test('a one-click join link pairs once without typing', async () => {
  window.location.hash = '#pair=012345';
  try {
    request.mockResolvedValue(response(401, {}));
    const ui = mount();
    expect(request).toHaveBeenCalledExactlyOnceWith('/__local/pair', {
      method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ code: '012345' }),
    });
    expect(ui.input.value).toBe('');
    await vi.waitFor(() => expect(ui.result.textContent).toContain('request a new code'));
  } finally {
    window.location.hash = '';
  }
});

test('a malformed join fragment never auto-submits', () => {
  window.location.hash = '#pair=abcdef';
  try {
    mount();
    expect(request).not.toHaveBeenCalled();
  } finally {
    window.location.hash = '';
  }
});
