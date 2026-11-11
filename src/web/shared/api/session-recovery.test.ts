import { afterEach, expect, test, vi } from 'vitest';
import { createSessionRecovery } from './session-recovery.ts';

afterEach(() => vi.useRealTimers());

test('a transient loss retries quietly and recovers without a dialog', async () => {
  vi.useFakeTimers();
  const retry = vi.fn().mockResolvedValue('ok');
  const recovery = createSessionRecovery({ pairUrl: '/', retry });
  recovery.report('reconnecting');
  expect(recovery.state.value).toBe('reconnecting');
  await vi.advanceTimersByTimeAsync(0);
  expect(retry).toHaveBeenCalledOnce();
  expect(recovery.state.value).toBe('ok');
});

test('retries with backoff and only surfaces unavailable once the budget is spent', async () => {
  const retry = vi.fn().mockResolvedValue('unavailable');
  const recovery = createSessionRecovery({ pairUrl: '/', retry, backoff: [0, 0, 0] });
  recovery.report('unavailable');
  await vi.waitFor(() => expect(recovery.state.value).toBe('unavailable'));
  expect(retry).toHaveBeenCalledTimes(4);
});

test('an expired cookie stops retrying and is not downgraded by later blips', async () => {
  vi.useFakeTimers();
  const retry = vi.fn().mockResolvedValue('expired');
  const recovery = createSessionRecovery({ pairUrl: '/', retry });
  recovery.report('reconnecting');
  await vi.advanceTimersByTimeAsync(0);
  expect(recovery.state.value).toBe('expired');
  recovery.report('unavailable');
  recovery.report('reconnecting');
  await vi.advanceTimersByTimeAsync(60_000);
  expect(recovery.state.value).toBe('expired');
  expect(retry).toHaveBeenCalledOnce();
});

test('a manual retry answers the click immediately instead of starting a budget', async () => {
  vi.useFakeTimers();
  const retry = vi.fn().mockResolvedValue('unavailable');
  const recovery = createSessionRecovery({ pairUrl: '/', retry, backoff: [10_000, 10_000] });
  recovery.report('unavailable');
  await vi.advanceTimersByTimeAsync(0);
  expect(recovery.state.value).toBe('reconnecting');
  await recovery.retryNow();
  expect(recovery.state.value).toBe('unavailable');
  expect(recovery.busy.value).toBe(false);
});

test('a successful manual retry clears the recovered state', async () => {
  vi.useFakeTimers();
  const retry = vi.fn().mockResolvedValueOnce('expired').mockResolvedValueOnce('ok');
  const recovery = createSessionRecovery({ pairUrl: '/remote/v1/join', retry });
  recovery.report('reconnecting');
  await vi.advanceTimersByTimeAsync(0);
  expect(recovery.state.value).toBe('expired');
  await recovery.retryNow();
  expect(recovery.state.value).toBe('ok');
  expect(recovery.pairUrl).toBe('/remote/v1/join');
});
