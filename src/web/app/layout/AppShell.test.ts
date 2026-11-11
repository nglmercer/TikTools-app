import { flushPromises } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createMemoryHistory, createRouter } from 'vue-router';
import { nextTick } from 'vue';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { controlClientKey } from '@/shared/api/use-control-client.ts';
import { diagnosticLog, diagnosticView } from '@/shared/diagnostics/log.ts';
import { createFakeControlClient } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
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
