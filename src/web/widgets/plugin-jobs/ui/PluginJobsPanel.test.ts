import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, it } from 'vitest';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import PluginJobsPanel from './PluginJobsPanel.vue';

enableAutoUnmount(afterEach);

const now = Math.floor(Date.now() / 1000);
const running = { id: 'j1', kind: 'install', pluginId: 'acme.tts', stage: 'download', status: 'running', progress: { done: 64, total: 100 }, createdAt: now, updatedAt: now };
const failed = { id: 'j2', kind: 'install', pluginId: 'acme.bad', stage: 'verify', status: 'failed', error: { code: 'unknown', message: 'Package rejected' }, createdAt: now, updatedAt: now + 5 };

async function mountJobs(jobs: unknown[]) {
  const { client, calls } = createFakeControlClient((method) => (method === 'plugins.jobs.list' ? { jobs } : {}));
  const stores = createStoreHarness(client);
  const store = stores.use(() => usePackagesStore());
  await store.initialize();
  const wrapper = mountWithPlugins(PluginJobsPanel, { global: { plugins: stores.plugins } });
  await flushPromises();
  return { wrapper, calls };
}

describe('PluginJobsPanel', () => {
  it('renders nothing while there is no job', async () => {
    const { wrapper } = await mountJobs([]);
    expect(wrapper.find('.plugin-jobs').exists()).toBe(false);
  });

  it('shows one thin strip per running job with the bar, the percentage and a cancel icon', async () => {
    const { wrapper, calls } = await mountJobs([running]);
    const strip = wrapper.get('.plugin-jobs__item');
    expect(strip.text()).toContain('acme.tts');
    expect(strip.text()).toContain('64%');
    expect(strip.get('progress').attributes('aria-label')).toBe('Downloading');
    // Icons only: no text button, and the stage is not spelled out twice.
    expect(strip.findAll('button')).toHaveLength(1);
    expect(strip.text()).not.toContain('Downloading');
    await strip.get('button[aria-label="Cancel"]').trigger('click');
    expect(calls.find((call) => call.method === 'plugins.jobs.cancel')?.params).toEqual({ jobId: 'j1' });
  });

  it('keeps a failed job readable until it is dismissed with its icon button', async () => {
    const { wrapper } = await mountJobs([failed]);
    const strip = wrapper.get('[role="alert"]');
    expect(strip.text()).toContain('Package rejected');
    expect(strip.find('progress').exists()).toBe(false);
    await strip.get('button[aria-label="Close"]').trigger('click');
    expect(wrapper.find('.plugin-jobs').exists()).toBe(false);
  });
});
