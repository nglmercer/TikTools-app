import { afterEach, expect, test } from 'vitest';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { DEFAULT_RESOLUTION_MODE, loadResolutionPreference, manualReview, resetResolutionPreference, resolutionMode, RESOLUTION_MODE_KEY, setResolutionMode } from '../../../../../../src/web/features/review-shared/model/resolution-preference.ts';

afterEach(() => resetResolutionPreference());

test('manual review is the default when nothing is stored or the read fails', async () => {
  expect(DEFAULT_RESOLUTION_MODE).toBe('manual');

  const { client } = createFakeControlClient(() => ({ state: {} }));
  await loadResolutionPreference(client);
  expect(resolutionMode.value).toBe('manual');
  expect(manualReview.value).toBe(true);

  resetResolutionPreference();
  const failing = createFakeControlClient(() => { throw new Error('offline'); });
  await loadResolutionPreference(failing.client);
  expect(resolutionMode.value).toBe('manual');
  expect(manualReview.value).toBe(true);
});

test('an unknown stored value falls back to manual instead of enabling automatic resolution', async () => {
  const { client } = createFakeControlClient(() => ({ state: { [RESOLUTION_MODE_KEY]: 'legacy' } }));
  await loadResolutionPreference(client);
  expect(resolutionMode.value).toBe('manual');
  expect(manualReview.value).toBe(true);
});

test('automatic resolution is an explicit opt-in stored value', async () => {
  const { client } = createFakeControlClient(() => ({ state: { [RESOLUTION_MODE_KEY]: 'automatic' } }));
  await loadResolutionPreference(client);
  expect(resolutionMode.value).toBe('automatic');
  expect(manualReview.value).toBe(false);
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
