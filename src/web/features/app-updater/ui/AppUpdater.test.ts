import { flushPromises } from '@vue/test-utils';
import { describe, expect, it, vi } from 'vitest';
import { createFakeControlClient, createStoreHarness } from '@/shared/lib/testing/fake-control-client.ts';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import type { UpdateStatus } from '../api/updater-types.ts';
import { useAppUpdaterStore } from '../model/updater-store.ts';
import AppUpdater from './AppUpdater.vue';
import BaseComboBox from '@/shared/ui/combo-box/BaseComboBox.vue';

function status(overrides: Partial<UpdateStatus> = {}): UpdateStatus {
  return { currentVersion: '4.0.0', availableVersion: null, channel: 'stable', state: 'idle', installationType: 'unknown',
    installSupported: false, checkSupported: true, downloadedBytes: 0, totalBytes: null, progress: null, releaseNotes: null, lastChecked: null, error: null, ...overrides };
}

describe('AppUpdater', () => {
  it('treats unsupported hosts as informational and keeps manual download available', () => {
    const { client } = createFakeControlClient(() => ({}));
    const harness = createStoreHarness(client);
    const store = harness.use(useAppUpdaterStore);
    store.error = 'desktop application updater is unavailable in this host';
    const wrapper = mountWithPlugins(AppUpdater, { global: { plugins: harness.plugins } });
    expect(wrapper.get('.diagnostic-action').classes()).toContain('diagnostic-action--info');
    expect(wrapper.text()).toContain('Automatic updates unavailable here');
    expect(wrapper.text()).not.toContain('desktop application updater is unavailable');
    expect(wrapper.find('a.app-updater__url').exists()).toBe(true);
  });

  it('shows the installed version and manual fallback, and saves the channel', async () => {
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'app.state.get') return { state: { 'app.update.preferences': JSON.stringify({ automaticChecks: false }) } };
      if (method === 'app.update.status') return status();
      return {};
    });
    const harness = createStoreHarness(client);
    const store = harness.use(useAppUpdaterStore);
    await store.initialize();
    const wrapper = mountWithPlugins(AppUpdater, { global: { plugins: harness.plugins } });
    expect(wrapper.text()).toContain('4.0.0');
    expect(wrapper.get('a.app-updater__url').attributes('href')).toBe('https://github.com/nglmercer/TikTools-app/releases');
    expect(wrapper.text()).not.toContain('Download update');
    wrapper.findComponent(BaseComboBox).vm.$emit('update:modelValue', 'beta');
    await flushPromises();
    await flushPromises();
    expect(calls.some((call) => call.method === 'app.state.set' && String(call.params?.value).includes('"channel":"beta"'))).toBe(true);
  });

  it('merges an early check event without overwriting saved preferences', async () => {
    let resolvePreferences: ((value: { state: Record<string, string> }) => void) | undefined;
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'app.state.get') return new Promise<{ state: Record<string, string> }>((resolve) => { resolvePreferences = resolve; });
      if (method === 'app.update.status') return status({ lastChecked: 150 });
      return {};
    });
    let emit: ((state: UpdateStatus) => void) | undefined;
    client.onTopic = (topic, handler) => {
      if (topic === 'app.update.stateChanged') emit = (state) => handler({ state } as never);
      return () => {};
    };
    const harness = createStoreHarness(client);
    const store = harness.use(useAppUpdaterStore);
    const initializing = store.initialize();
    emit?.(status({ lastChecked: 200 }));
    await flushPromises();
    expect(calls.some((call) => call.method === 'app.state.set')).toBe(false);
    resolvePreferences?.({ state: { 'app.update.preferences': '{"channel":"beta","automaticChecks":false,"notifications":false,"lastChecked":100}' } });
    await initializing;
    await flushPromises();
    expect(store.preferences).toMatchObject({ channel: 'beta', automaticChecks: false, notifications: false, lastChecked: 200 });
    const write = calls.find((call) => call.method === 'app.state.set');
    expect(JSON.parse(String(write?.params?.value))).toMatchObject({ channel: 'beta', automaticChecks: false, notifications: false, lastChecked: 200 });
  });

  it('keeps a newer event received while status is loading', async () => {
    let resolveStatus: ((value: UpdateStatus) => void) | undefined;
    const { client } = createFakeControlClient((method) => {
      if (method === 'app.state.get') return { state: { 'app.update.preferences': '{"automaticChecks":false}' } };
      if (method === 'app.update.status') return new Promise<UpdateStatus>((resolve) => { resolveStatus = resolve; });
      return {};
    });
    let emit: ((state: UpdateStatus) => void) | undefined;
    client.onTopic = (topic, handler) => {
      if (topic === 'app.update.stateChanged') emit = (state) => handler({ state } as never);
      return () => {};
    };
    const harness = createStoreHarness(client);
    const store = harness.use(useAppUpdaterStore);
    const initializing = store.initialize();
    await flushPromises();
    emit?.(status({ state: 'downloading', downloadedBytes: 12, checkSupported: true }));
    resolveStatus?.(status({ state: 'idle', checkSupported: false }));
    await initializing;
    expect(store.status?.state).toBe('downloading');
    expect(store.status?.checkSupported).toBe(true);
  });

  it('renders unknown download length without inventing a percentage', async () => {
    const { client } = createFakeControlClient((method) => {
      if (method === 'app.state.get') return { state: { 'app.update.preferences': '{"automaticChecks":false}' } };
      if (method === 'app.update.status') return status({ state: 'downloading', downloadedBytes: 12 });
      return {};
    });
    const harness = createStoreHarness(client);
    await harness.use(useAppUpdaterStore).initialize();
    const wrapper = mountWithPlugins(AppUpdater, { global: { plugins: harness.plugins } });
    expect(wrapper.text()).toContain('Downloading update');
    expect(wrapper.get('progress').attributes('value')).toBeUndefined();
  });

  it('renders progress delivered by the update event stream', async () => {
    const { client } = createFakeControlClient((method) => {
      if (method === 'app.state.get') return { state: { 'app.update.preferences': '{"automaticChecks":false}' } };
      if (method === 'app.update.status') return status();
      return {};
    });
    let emit: ((state: UpdateStatus) => void) | undefined;
    client.onTopic = (topic, handler) => {
      if (topic === 'app.update.stateChanged') emit = (state) => handler({ state } as never);
      return () => {};
    };
    const harness = createStoreHarness(client);
    await harness.use(useAppUpdaterStore).initialize();
    const wrapper = mountWithPlugins(AppUpdater, { global: { plugins: harness.plugins } });
    emit?.(status({ state: 'downloading', downloadedBytes: 50, totalBytes: 100, progress: 50 }));
    await flushPromises();
    expect(wrapper.text()).toContain('50%');
    expect(wrapper.get('progress').attributes('value')).toBe('50');
  });

  it('hides a ready update when the selected channel changes', async () => {
    const { client } = createFakeControlClient((method) => {
      if (method === 'app.state.get') return { state: { 'app.update.preferences': '{"automaticChecks":false,"channel":"beta"}' } };
      if (method === 'app.update.status') return status({ channel: 'beta', state: 'readyToInstall', installSupported: true });
      return {};
    });
    const harness = createStoreHarness(client);
    await harness.use(useAppUpdaterStore).initialize();
    const wrapper = mountWithPlugins(AppUpdater, { global: { plugins: harness.plugins } });
    expect(wrapper.text()).toContain('Install and restart');
    wrapper.findComponent(BaseComboBox).vm.$emit('update:modelValue', 'stable');
    await flushPromises();
    expect(wrapper.text()).not.toContain('Install and restart');
    expect(wrapper.text()).toContain('Check for updates');
  });

  it('offers a retry after a download failure', async () => {
    const { client, calls } = createFakeControlClient((method) => {
      if (method === 'app.state.get') return { state: { 'app.update.preferences': '{"automaticChecks":false}' } };
      if (method === 'app.update.status') return status({ state: 'failed', installSupported: true, error: { code: 'download_failed', message: 'Download failed' } });
      return {};
    });
    const harness = createStoreHarness(client);
    await harness.use(useAppUpdaterStore).initialize();
    const wrapper = mountWithPlugins(AppUpdater, { global: { plugins: harness.plugins } });
    const retry = wrapper.findAll('button').find((button) => button.text().includes('Retry download'));
    expect(retry).toBeDefined();
    await retry!.trigger('click');
    expect(calls.some((call) => call.method === 'app.update.download')).toBe(true);
  });

  it('requests native confirmation without passing an installer path', async () => {
    const { client } = createFakeControlClient((method) => {
      if (method === 'app.state.get') return { state: { 'app.update.preferences': '{"automaticChecks":false}' } };
      if (method === 'app.update.status') return status({ state: 'readyToInstall', installSupported: true });
      return {};
    });
    const harness = createStoreHarness(client);
    await harness.use(useAppUpdaterStore).initialize();
    const postMessage = vi.fn();
    const previous = window.ipc;
    window.ipc = { postMessage };
    try {
      const wrapper = mountWithPlugins(AppUpdater, { global: { plugins: harness.plugins } });
      const button = wrapper.findAll('button').find((item) => item.text().includes('Install and restart'));
      expect(button).toBeDefined();
      await button!.trigger('click');
      expect(JSON.parse(postMessage.mock.calls[0]![0] as string)).toEqual({ type: 'app-update-install', locale: 'en' });
    } finally {
      window.ipc = previous;
    }
  });
});
