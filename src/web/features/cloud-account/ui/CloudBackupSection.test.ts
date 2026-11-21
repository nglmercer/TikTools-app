import { testMessage } from '@/shared/i18n/test-message.ts';
import { flushPromises, type VueWrapper } from '@vue/test-utils';
import { afterEach, expect, it, vi } from 'vitest';
import type { AccountPlan, StoredBackup } from '@/entities/cloud-account/api/account-types.ts';
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
const stored: StoredBackup = {
  id: 'b1', name: 'Before trip', createdAt: new Date().toISOString(), sizeBytes: 2048,
  workspaceSync: 2, library: 1, mediaLibrary: 0, assets: 0,
};
const plan = (backups: number): AccountPlan => ({
  plan: 'free',
  limits: { maxRecords: 1000, maxBytes: 8_388_608, maxAssets: 200, maxAssetsBytes: 33_554_432, maxOverlays: 50, maxOverlayBytes: 16_777_216, maxBackups: 3, maxBackupBytes: 16_777_216 },
  usage: { records: 0, recordBytes: 0, assets: 0, assetBytes: 0, overlays: 0, overlayBytes: 0, backups, backupBytes: 2048 * backups },
});

function mountSection(handler: (method: string, params: unknown) => unknown) {
  const { client, calls } = createFakeControlClient(handler as never);
  const wrapper = mountWithPlugins(CloudBackupSection, {
    global: { plugins: storePlugins(client) },
    attachTo: document.body,
  });
  return { wrapper, calls };
}

const button = (wrapper: VueWrapper, label: string) =>
  wrapper.findAll('button').find((node) => node.text().startsWith(label))!;

async function pickFile(wrapper: VueWrapper, content: string, name: string): Promise<void> {
  const input = wrapper.get('input[type="file"]');
  const file = new File([content], name, { type: 'application/json' });
  Object.defineProperty(input.element, 'files', { value: [file], configurable: true });
  await input.trigger('change');
  await flushPromises();
}

it('shows the cloud tab first with a suggested name, the local tab tools and the plan quota', async () => {
  const { wrapper } = mountSection((method) => {
    if (method === 'cloud.backup.list') return { items: [] };
    if (method === 'cloud.plan.get') return plan(1);
    throw new Error(`unexpected ${method}`);
  });
  await flushPromises();
  expect(wrapper.text()).toContain(t('account.backup.saveCloud'));
  expect(wrapper.text()).toContain(t('account.backup.cloudHint'));
  expect((wrapper.get('.backup-section__name input').element as HTMLInputElement).value).toMatch(/^Backup \d{4}-\d{2}-\d{2} \d{2}:\d{2}$/);
  expect(wrapper.text()).toContain(t('account.backup.emptyShort'));
  expect(wrapper.text()).not.toContain('1 of 3 backups');
  expect(wrapper.text()).not.toContain(t('account.backup.uploadFile'));
  await button(wrapper, t('account.backup.tabLocal')).trigger('click');
  expect(wrapper.text()).toContain(t('account.backup.localHint'));
  expect(wrapper.text()).toContain(t('account.backup.uploadFile'));
  expect(wrapper.text()).toContain(t('account.backup.downloadFile'));
  wrapper.unmount();
});

it('exports the current data as a JSON file', async () => {
  const createObjectURL = vi.fn(() => 'blob:backup');
  vi.stubGlobal('URL', { ...URL, createObjectURL, revokeObjectURL: vi.fn() });
  const { wrapper } = mountSection((method) => {
    if (method === 'cloud.backup.list') return { items: [] };
    if (method === 'cloud.plan.get') return plan(0);
    if (method === 'cloud.backup.create') return manifest;
    throw new Error(`unexpected ${method}`);
  });
  await flushPromises();
  await button(wrapper, t('account.backup.tabLocal')).trigger('click');
  await button(wrapper, t('account.backup.downloadFile')).trigger('click');
  await flushPromises();
  expect(createObjectURL).toHaveBeenCalledOnce();
  expect(wrapper.text()).toContain(t('account.backup.done'));
  wrapper.unmount();
  vi.unstubAllGlobals();
});

it('lists stored backups and saves a named one from the account data', async () => {
  const { wrapper, calls } = mountSection((method) => {
    if (method === 'cloud.backup.list') return { items: [stored] };
    if (method === 'cloud.plan.get') return plan(1);
    if (method === 'cloud.backup.createNamed') return { ...stored, id: 'b2', name: 'Nightly' };
    throw new Error(`unexpected ${method}`);
  });
  await flushPromises();
  expect(wrapper.text()).toContain('Before trip');
  expect(wrapper.text()).toContain('2.0 KB');

  await wrapper.get('form').trigger('submit');
  await flushPromises();
  const call = calls.find((entry) => entry.method === 'cloud.backup.createNamed')!;
  expect(call.params).toEqual({ name: expect.stringMatching(/^Backup \d{4}-\d{2}-\d{2} \d{2}:\d{2}$/) });
  expect(wrapper.text()).toContain('Nightly');
  expect(wrapper.text()).toContain(t('account.backup.created'));
  wrapper.unmount();
});

it('uploads a picked backup file to the account with its contents', async () => {
  const { wrapper, calls } = mountSection((method) => {
    if (method === 'cloud.backup.list') return { items: [] };
    if (method === 'cloud.plan.get') return plan(0);
    if (method === 'cloud.backup.preview') return { mode: 'replace', dryRun: true, applied: false, legacySync: 0, workspaceSync: 4, library: 1, mediaLibrary: 0, assets: 2 };
    if (method === 'cloud.backup.createNamed') return { ...stored, name: 'laptop' };
    throw new Error(`unexpected ${method}`);
  });
  await flushPromises();
  await button(wrapper, t('account.backup.tabLocal')).trigger('click');
  await pickFile(wrapper, JSON.stringify(manifest), 'laptop.json');
  expect(document.body.textContent).toContain('4 records, 1 documents and 2 files');

  const input = document.body.querySelector<HTMLInputElement>('dialog input[type="text"]')!;
  expect(input.value).toBe('laptop');
  const save = [...document.body.querySelectorAll('dialog button')].find((node) => node.textContent?.trim() === t('account.backup.saveFile'))!;
  (save as HTMLButtonElement).click();
  await flushPromises();
  const call = calls.find((entry) => entry.method === 'cloud.backup.createNamed')!;
  expect(call.params).toEqual({ name: 'laptop', manifest });
  expect(wrapper.text()).toContain(t('account.backup.uploaded'));
  wrapper.unmount();
});

it('rejects a file that is not a backup', async () => {
  const { wrapper, calls } = mountSection((method) => {
    if (method === 'cloud.backup.list') return { items: [] };
    if (method === 'cloud.plan.get') return plan(0);
    throw new Error(`unexpected ${method}`);
  });
  await flushPromises();
  await button(wrapper, t('account.backup.tabLocal')).trigger('click');
  await pickFile(wrapper, '{"hello":true}', 'notes.json');
  expect(wrapper.text()).toContain(t('account.backup.invalid'));
  expect(calls.some((entry) => entry.method === 'cloud.backup.preview')).toBe(false);
  wrapper.unmount();
});

it('blocks creating a backup once the plan limit is reached', async () => {
  const { wrapper } = mountSection((method) => {
    if (method === 'cloud.backup.list') return { items: [stored] };
    if (method === 'cloud.plan.get') return plan(3);
    throw new Error(`unexpected ${method}`);
  });
  await flushPromises();
  expect(button(wrapper, t('account.backup.saveCloud')).attributes('disabled')).toBeDefined();
  expect(wrapper.text()).toContain(t('account.backup.full'));
  wrapper.unmount();
});
