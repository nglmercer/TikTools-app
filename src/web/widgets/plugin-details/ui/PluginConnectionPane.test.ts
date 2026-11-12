import { enableAutoUnmount } from '@vue/test-utils';
import { afterEach, expect, it, vi } from 'vitest';
import { shallowRef } from 'vue';
import type { PluginControls } from '@/features/manage-plugins/model/use-plugin-controls.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import PluginConnectionPane from './PluginConnectionPane.vue';

enableAutoUnmount(afterEach);

it('flushes pending settings before asking the parent to probe the connection', async () => {
  vi.useFakeTimers();
  const operations: string[] = [];
  const text = (value: string) => ({ default: value, i18key: '' });
  const save = vi.fn<PluginControls['saveSettings']>(() => { operations.push('save'); });
  const plugins = {
    settings: shallowRef({ server: {
      schema: { type: 'object', properties: {
        serverUrl: { type: 'string', title: 'Server URL', format: 'uri' },
      } },
      values: { serverUrl: 'http://127.0.0.1:8000' },
    } }),
    actionOptions: shallowRef({}),
    provision: shallowRef({}),
    saveSettings: save,
    loadSettings: vi.fn<PluginControls['loadSettings']>(),
    loadOptions: vi.fn<PluginControls['loadOptions']>(),
    testConnection: vi.fn<PluginControls['testConnection']>(),
    importToken: vi.fn<PluginControls['importToken']>(),
    provisionToken: vi.fn<PluginControls['provisionToken']>(),
    openMediaPicker: vi.fn<PluginControls['openMediaPicker']>(),
  };
  const wrapper = mountWithPlugins(PluginConnectionPane, { props: {
    descriptor: { id: 'server', name: text('Server'), version: '1', description: text(''), dependency: text(''), permissions: [], actionTypeIds: [], eventTypeIds: [] },
    name: 'Server', connecting: false, connection: { ok: false, at: 1, latencyMs: 0 }, plugins,
    onTest: () => { operations.push('probe'); },
  } });
  try {
    await wrapper.get('.connection-editor input').setValue('http://127.0.0.1:9000');
    expect(save).not.toHaveBeenCalled();
    await wrapper.get('.plugin-details__test').trigger('click');
    expect(operations).toEqual(['save', 'probe']);
    expect(save).toHaveBeenCalledWith('server', { serverUrl: 'http://127.0.0.1:9000' });
    vi.advanceTimersByTime(1000);
    expect(save).toHaveBeenCalledTimes(1);
  } finally {
    wrapper.unmount();
    vi.useRealTimers();
  }
});
