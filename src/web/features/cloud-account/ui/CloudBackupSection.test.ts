import { testMessage } from '@/shared/i18n/test-message.ts';
import { flushPromises } from '@vue/test-utils';
import { afterEach, expect, it, vi } from 'vitest';
import { activateLocale } from '@/shared/i18n/i18n.ts';
import { createFakeControlClient, storePlugins } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import CloudBackupSection from './CloudBackupSection.vue';

const t = (key: string): string => testMessage('en', key);
afterEach(async () => { await activateLocale('en'); });
const manifest = {
  schemaVersion: 1,
  createdAt: new Date().toISOString(),
  workspaceSync: { schemaVersion: 2, cursor: 0, records: [] },
};

it('offers cloud and local file modes', async () => {
  const { client } = createFakeControlClient((method) => {
    if (method === 'cloud.backup.list') return { items: [] };
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(CloudBackupSection, {
    global: { plugins: storePlugins(client) },
  });
  await flushPromises();
  expect(wrapper.text()).toContain(t('account.backup.modeCloud'));
  expect(wrapper.text()).toContain(t('account.backup.modeFile'));
  expect(wrapper.text()).toContain(t('account.backup.cloudHint'));
  expect(wrapper.text()).toContain(t('account.backup.saveCloud'));
  wrapper.unmount();
});

it('downloads a local JSON backup in file mode', async () => {
  const { client } = createFakeControlClient((method) => {
    if (method === 'cloud.backup.list') return { items: [] };
    if (method === 'cloud.backup.create') return manifest;
    throw new Error(`unexpected ${method}`);
  });
  const createObjectURL = vi.fn(() => 'blob:backup');
  const revokeObjectURL = vi.fn();
  vi.stubGlobal('URL', { ...URL, createObjectURL, revokeObjectURL });

  const wrapper = mountWithPlugins(CloudBackupSection, {
    global: { plugins: storePlugins(client) },
  });
  await flushPromises();
  await wrapper.findAll('button').find((button) => button.text() === t('account.backup.modeFile'))!.trigger('click');
  await wrapper.findAll('button').find((button) => button.text() === t('account.backup.downloadFile'))!.trigger('click');
  await flushPromises();
  expect(createObjectURL).toHaveBeenCalledOnce();
  expect(wrapper.text()).toContain(t('account.backup.done'));
  wrapper.unmount();
  vi.unstubAllGlobals();
});

it('lists stored cloud backups and uploads a named one', async () => {
  const stored = {
    id: 'b1', name: 'Before trip', createdAt: new Date().toISOString(), sizeBytes: 10,
    workspaceSync: 2, library: 1, mediaLibrary: 0, assets: 0,
  };
  const { client } = createFakeControlClient((method) => {
    if (method === 'cloud.backup.list') return { items: [stored] };
    if (method === 'cloud.backup.createNamed') return { ...stored, name: 'Nightly' };
    throw new Error(`unexpected ${method}`);
  });
  const wrapper = mountWithPlugins(CloudBackupSection, {
    global: { plugins: storePlugins(client) },
  });
  await flushPromises();
  expect(wrapper.text()).toContain('Before trip');
  expect(wrapper.text()).toContain(t('account.backup.saveCloud'));

  await wrapper.get('.backup-section__name').setValue('Nightly');
  await wrapper.findAll('button').find((button) => button.text() === t('account.backup.saveCloud'))!.trigger('click');
  await flushPromises();
  expect(wrapper.text()).toContain('Nightly');
  wrapper.unmount();
});
