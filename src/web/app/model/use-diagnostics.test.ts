import { flushPromises } from '@vue/test-utils';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { defineComponent, h, nextTick } from 'vue';
import { enableAutoUnmount } from '@vue/test-utils';
import { useAccountStore } from '@/entities/cloud-account/model/account-store.ts';
import { useCatalogStore } from '@/entities/catalog/model/catalog-store.ts';
import { useTeamsStore } from '@/entities/cloud-teams/model/teams-store.ts';
import { diagnosticLog } from '@/shared/diagnostics/log.ts';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { useDiagnostics } from './use-diagnostics.ts';

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
    stores.use(() => useTeamsStore()).error = 'The relay refused the ticket';
    await nextTick();
    const sources = diagnosticLog.entries.value.map((entry) => `${entry.source}:${entry.code}`);
    expect(sources).toContain('catalog:catalog.error');
    expect(sources).toContain('collaboration:teams.error');
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

  it('keeps the request id and Cloud code of a failure so support can correlate it', async () => {
    const stores = await mountDiagnostics();
    const teams = stores.use(() => useTeamsStore());
    teams.errorDetail = { code: 'offline', requestId: 'req-42', retryable: true };
    teams.error = 'the host is offline';
    await nextTick();
    const entry = diagnosticLog.entries.value.find((candidate) => candidate.source === 'collaboration');
    expect(entry).toMatchObject({ code: 'teams.error:offline', requestId: 'req-42' });
  });
});
