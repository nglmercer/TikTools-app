import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, expect, it } from 'vitest';
import type { PackagePreview } from '@/entities/plugin-package/api/package-types.ts';
import { usePackagesStore } from '@/entities/plugin-package/model/packages-store.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import InstallPluginDialog from './InstallPluginDialog.vue';

enableAutoUnmount(afterEach);

const preview: PackagePreview = {
  id: 'demo', name: 'Demo', version: '2.0.0', installedVersion: '1.0.0',
  runtime: 'process', securityModel: 'isolated', permissions: ['audio.output'], capabilities: [],
  size: 2048, digest: 'a'.repeat(64), origin: { kind: 'file' }, signature: { status: 'unsigned' },
  change: 'update', requiresSideloadConsent: true, metadata: {}, consentToken: 'first',
  permissionDiff: { addedPermissions: ['audio.output'], removedPermissions: ['old.permission'], addedCapabilities: [], removedCapabilities: [] },
};

it('requires fresh consent for a changed preview and prevents confirming incompatible packages', async () => {
  const { client, calls } = createFakeControlClient(() => { throw new Error('unexpected request'); });
  const stores = createStoreHarness(client);
  const packages = stores.use(usePackagesStore);
  packages.review = { open: true, mode: 'update', loading: false, submitting: false, spec: { kind: 'file', path: '/demo.plugin' }, preview, error: null };
  const wrapper = mountWithPlugins(InstallPluginDialog, { global: { plugins: stores.plugins } });
  const confirm = () => wrapper.findAll('button').find((button) => button.text() === 'Update')!;
  expect(wrapper.get('.install-dialog__name').text()).toContain('Demo');
  expect(wrapper.get('.install-dialog__added').text()).toContain('Play audio');
  expect(wrapper.get('.install-dialog__removed').text()).toContain('old.permission');
  expect(confirm().attributes('disabled')).toBeDefined();
  await wrapper.get('input[type="checkbox"]').setValue(true);
  expect(confirm().attributes('disabled')).toBeUndefined();

  packages.review.preview = { ...preview, version: '2.1.0', consentToken: 'second', permissions: [] };
  await wrapper.vm.$nextTick();
  expect(wrapper.get('.install-dialog__name').text()).toContain('2.1.0');
  expect((wrapper.get('input[type="checkbox"]').element as HTMLInputElement).checked).toBe(false);
  expect(confirm().attributes('disabled')).toBeDefined();
  await wrapper.get('input[type="checkbox"]').setValue(true);
  packages.review.preview = { ...packages.review.preview, incompatibility: { kind: 'runtime', runtime: 'unsupported' } };
  await wrapper.vm.$nextTick();
  expect(wrapper.find('input[type="checkbox"]').exists()).toBe(false);
  expect(confirm().attributes('disabled')).toBeDefined();
  await confirm().trigger('click');
  expect(calls).toEqual([]);
});
