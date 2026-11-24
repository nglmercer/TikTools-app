import { flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { defineComponent, h, nextTick } from 'vue';
import { enableAutoUnmount } from '@vue/test-utils';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useCatalogStore } from '@/entities/catalog/model/catalog-store.ts';
import { diagnosticLog } from '@/shared/diagnostics/log.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { useDiagnostics } from '../../../../../src/web/app/model/use-diagnostics.ts';

enableAutoUnmount(afterEach);
beforeEach(() => diagnosticLog.clear());

async function mountDiagnostics() {
  const { client } = createFakeControlClient(() => ({}));
  const stores = createStoreHarness(client);
  const Host = defineComponent({ setup() { useDiagnostics(); return () => h('div'); } });
  mountWithPlugins(Host, { global: { plugins: stores.plugins } });
  await flushPromises();
  return stores;
}

describe('useDiagnostics', () => {
  it('records Cloud failures under their own sources instead of leaving them on the panels', async () => {
    const stores = await mountDiagnostics();
    stores.use(() => useCatalogStore()).error = 'The catalog service did not answer';
    await nextTick();
    const sources = diagnosticLog.entries.value.map((entry) => `${entry.source}:${entry.code}`);
    expect(sources).toContain('catalog:catalog.error');
  });

  it('records the Cloud diagnostics as a structured entry in the log', async () => {
    const stores = await mountDiagnostics();
    const account = stores.use(() => useAccountStore());
    account.status = { origin: 'http://127.0.0.1:3000', state: 'signed-in', accountId: 'acc-1', persistence: 'memory-only' } as never;
    await nextTick();
    const entry = diagnosticLog.entries.value.filter(item => item.code === 'account.status').at(-1);
    expect(entry).toBeTruthy();
    expect(entry!.level).toBe('info');
    expect(entry!.fields.map(field => field.value)).toEqual(expect.arrayContaining(['signed-in', 'acc-1']));
    expect(entry!.details).toBe('');
  });

  it('drops the history of an account when another one signs in', async () => {
    const stores = await mountDiagnostics();
    const account = stores.use(() => useAccountStore());
    const status = (accountId: string | null) => ({ origin: 'https://cloud.example', state: 'signed-in', accountId, persistence: 'memory' }) as never;
    account.status = status('account-a');
    stores.use(() => useCatalogStore()).error = 'private detail of account a';
    await nextTick();
    expect(diagnosticLog.entries.value.some((entry) => entry.message.includes('account a'))).toBe(true);
    account.status = status('account-b');
    await nextTick();
    expect(diagnosticLog.entries.value.some((entry) => entry.message.includes('account a'))).toBe(false);
  });

});
