import { flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createMemoryHistory, createRouter } from 'vue-router';
import { nextTick } from 'vue';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { diagnosticLog, diagnosticView } from '@/shared/diagnostics/log.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import { nativeDefinitionHost } from '@/entities/automation/testing/native-definition-fixture.ts';
import { importLibraryDocument } from '@/entities/cloud-bundles/lib/library-import.ts';
import AppShell from './AppShell.vue';

afterEach(() => {
  diagnosticView.open.value = false;
});

async function mountShell() {
  const { client } = createFakeControlClient(() => ({ methods: [], items: [] }));
  const pinia = createPinia();
  setActivePinia(pinia);
  const router = createRouter({
    history: createMemoryHistory(),
    routes: [{ path: '/', component: { template: '<div />' } }],
  });
  await router.push('/');
  const wrapper = mountWithPlugins(AppShell, {
    attachTo: document.body,
    global: {
      plugins: [pinia, router, { install: (app) => void app.provide(controlClientKey, client) }],
      stubs: { RouterView: true },
    },
  });
  await flushPromises();
  return wrapper;
}

describe('AppShell diagnostics', () => {
  it('keeps the diagnostics panel out of the DOM while it is closed', async () => {
    const wrapper = await mountShell();
    expect(document.querySelector('.diagnostic-panel')).toBeNull();
    expect(document.querySelector('.logs')).toBeNull();
    wrapper.unmount();
  });

  it('mounts the panel when opened and unmounts it on close without clearing the log', async () => {
    diagnosticLog.record({ source: 'transport', level: 'error', code: 'probe', message: 'captured before opening' });
    const wrapper = await mountShell();
    diagnosticView.open.value = true;
    await nextTick();
    await flushPromises();
    // The panel is an async chunk; wait for it to resolve rather than for a fixed number of ticks.
    await vi.waitFor(() => expect(document.querySelector('.diagnostic-panel')).not.toBeNull());
    const beforeClose = diagnosticLog.entries.value.length;
    diagnosticView.open.value = false;
    await nextTick();
    await vi.waitFor(() => expect(document.querySelector('.diagnostic-panel')).toBeNull());
    expect(diagnosticLog.entries.value.length).toBe(beforeClose);
    wrapper.unmount();
  });
});

const libraryDocumentId = '00000000-0000-4000-8000-000000000001';

describe('AppShell library review', () => {
  const template = { templateVersion: 1, id: 'shell-template', title: 'Shell template', description: '', icon: 'gift',
    actions: [{ name: 'act', typeId: 'core.log', config: {} }], event: { name: 'evt', trigger: 'tiktok.chat' } };
  async function mountOwnerShell() {
    const native = nativeDefinitionHost((method, params, options) => {
      if (method === 'automation.snapshot') { options?.onMetadata?.({ resourceVersions: [] }); return { actions: [], events: [], plugins: [], actionTypes: [], translations: {} }; }
      if (method === 'app.state.get') return { state: {} };
      if (method === 'cloudLibrary.read') return { meta: { id: libraryDocumentId, revision: '1', sha256: 'a'.repeat(64) }, document: template };
      if (method === 'automation.create') {
        options?.onMetadata?.({ resourceVersions: [{ kind: 'automation', scope: params?.kind as string, id: (params?.record as { id: string }).id, revision: '1' }] });
        return params?.record;
      }
      return { methods: [], items: [] };
    });
    const { client, calls } = createFakeControlClient(native.handle);
    const pinia = createPinia();
    setActivePinia(pinia);
    const router = createRouter({
      history: createMemoryHistory(),
      routes: ['/', '/settings', '/plugins'].map((path) => ({ path, component: { template: '<div />' } })),
    });
    await router.push('/plugins');
    const wrapper = mountWithPlugins(AppShell, {
      attachTo: document.body,
      global: { plugins: [pinia, router, { install: (app) => void app.provide(controlClientKey, client) }], stubs: { RouterView: true } },
    });
    await flushPromises();
    return { wrapper, router, client, calls };
  }
  const dialogs = () => document.querySelectorAll('[data-test="library-confirm"]');

  it('shows one review for an import started away from Automations and keeps it across routes until the owner decides', async () => {
    const shell = await mountOwnerShell();
    const identity = { origin: 'https://cloud.example', accountId: 'alice' };
    const environment = { client: shell.client, identity: () => identity, stamp: () => 0, isCurrent: () => true };
    const id = libraryDocumentId;
    let settled = false;
    const outcome = importLibraryDocument(environment, 'template', { id, revision: 1 }).then(() => { settled = true; });
    await flushPromises();
    expect(dialogs()).toHaveLength(1);
    await shell.router.push('/settings'); await flushPromises();
    await shell.router.push('/'); await flushPromises();
    expect(dialogs()).toHaveLength(1);
    expect(settled).toBe(false);
    (dialogs()[0] as HTMLButtonElement).click();
    await flushPromises(); await outcome;
    expect(settled).toBe(true);
    expect(dialogs()).toHaveLength(0);
    expect(shell.calls.filter((call) => call.method === 'automation.library.submit')).toHaveLength(1);
    shell.wrapper.unmount();
  });
});
