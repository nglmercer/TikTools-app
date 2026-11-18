import { afterEach, expect, test } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { DEFAULT_RESOLUTION_MODE, loadResolutionPreference, manualReview, resetResolutionPreference, resolutionMode, RESOLUTION_MODE_KEY, setResolutionMode } from './resolution-preference.ts';

afterEach(() => resetResolutionPreference());

test('defaults to automatic when nothing is stored or the read fails', async () => {
  const { client } = createFakeControlClient(() => ({ state: {} }));
  await loadResolutionPreference(client);
  expect(resolutionMode.value).toBe(DEFAULT_RESOLUTION_MODE);
  expect(manualReview.value).toBe(false);

  resetResolutionPreference();
  const failing = createFakeControlClient(() => { throw new Error('offline'); });
  await loadResolutionPreference(failing.client);
  expect(resolutionMode.value).toBe('automatic');
});

test('loads and persists the advanced manual mode', async () => {
  let stored = 'manual';
  const { client, calls } = createFakeControlClient((method, params) => {
    if (method === 'app.state.get') return { state: { [RESOLUTION_MODE_KEY]: stored } };
    if (method === 'app.state.set') { stored = String(params?.value); return {}; }
    throw new Error(`unexpected ${method}`);
  });
  await loadResolutionPreference(client);
  expect(resolutionMode.value).toBe('manual');
  expect(manualReview.value).toBe(true);

  await setResolutionMode(client, 'automatic');
  expect(stored).toBe('automatic');
  expect(resolutionMode.value).toBe('automatic');
  expect(calls.filter((call) => call.method === 'app.state.set')).toHaveLength(1);
});
