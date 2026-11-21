import { testMessage } from '@/shared/i18n/test-message.ts';
import { flushPromises } from '@vue/test-utils';
import { afterEach, expect, it, vi } from 'vitest';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudBackupCard from './CloudBackupCard.vue';

const t = (key: string): string => testMessage('en', key);
afterEach(async () => { await activateLocale('en'); });

it('creates a portable backup and downloads it', async () => {
  const manifest = {
    schemaVersion: 1,
    createdAt: new Date().toISOString(),
    workspaceSync: { schemaVersion: 2, cursor: 0, records: [] },
  };
  const { client, calls } = createFakeControlClient((method) => {
    if (method === 'cloud.backup.create') return manifest;
    throw new Error(`unexpected ${method}`);
  });
  const createObjectURL = vi.fn(() => 'blob:backup');
  const revokeObjectURL = vi.fn();
  vi.stubGlobal('URL', { ...URL, createObjectURL, revokeObjectURL });

  const wrapper = mountWithPlugins(CloudBackupCard, {
    global: { plugins: storePlugins(client) },
  });
  await flushPromises();
  expect(wrapper.text()).toContain(t('account.backup.title'));
  await wrapper.findAll('button').find((button) => button.text() === t('account.backup.create'))!.trigger('click');
  await flushPromises();
  expect(calls.some((call) => call.method === 'cloud.backup.create')).toBe(true);
  expect(createObjectURL).toHaveBeenCalledOnce();
  expect(wrapper.text()).toContain(t('account.backup.done'));
  wrapper.unmount();
  vi.unstubAllGlobals();
});
