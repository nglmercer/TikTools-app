import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, expect, test } from 'vitest';
import { mountWithPlugins } from '@/shared/lib/testing/mount.ts';
import PluginConnectionEditor from './PluginConnectionEditor.vue';

enableAutoUnmount(afterEach);
const settings = {
  schema: { type: 'object', properties: {
    serverUrl: { type: 'string', format: 'uri', title: 'Server URL', default: 'http://127.0.0.1:17842' },
    apiToken: { type: 'string', title: 'API token', secret: true },
  } },
  values: { serverUrl: 'http://127.0.0.1:17842', apiToken: '••••••••' },
};
function mountEditor(required: boolean, authorized: boolean) {
  return mountWithPlugins(PluginConnectionEditor, { attachTo: document.body, props: {
    pluginId: 'sonicboom.server', pluginName: 'SonicBoom', actionOptions: {}, settings,
    supportsProvisioning: true,
    connection: { ok: authorized, reachable: true, latencyMs: 5, at: 1,
      serverInfo: { authRequired: required, authorized, modelStatus: 'downloading', setupComplete: true, playback: true } },
  } });
}

test('a local server without authentication stays connected while downloading its model', () => {
  const wrapper = mountEditor(false, true);
  expect(wrapper.text()).toContain('Connected to http://127.0.0.1:17842');
  expect(wrapper.text()).toContain('Downloading model');
  expect(wrapper.text()).toContain('No token needed');
  expect(wrapper.text()).not.toContain('Sign in to connect');
  expect(wrapper.find('input[type="password"]').exists()).toBe(false);
});

test('connect opens the admin login when the server requires authorization', async () => {
  const wrapper = mountEditor(true, false);
  await wrapper.findAll('button').find(button => button.text() === 'Connect SonicBoom')!.trigger('click');
  await wrapper.setProps({ connection: { ...wrapper.props('connection')!, at: 2 } });
  await flushPromises();
  const username = document.querySelector<HTMLInputElement>('#provision-user');
  expect(username?.value).toBe('admin');
  expect(document.querySelector('#provision-password')).not.toBeNull();
  expect(wrapper.text()).not.toContain('No token needed');
});

test('manual token entry is available under advanced settings and preserves the saved value', async () => {
  const wrapper = mountEditor(true, false);
  expect(wrapper.find('input[type="password"]').exists()).toBe(false);
  await wrapper.findAll('button').find(button => button.text() === 'Advanced settings')!.trigger('click');
  expect(wrapper.find('input[type="password"]').exists()).toBe(true);
  expect(wrapper.emitted('save')).toBeUndefined();
});

test('discovery completes before opening the sign-in dialog on a first connection', async () => {
  const wrapper = mountEditor(false, false);
  await wrapper.setProps({ connection: { ok: false, latencyMs: 0, at: 2 } });
  await wrapper.findAll('button').find(button => button.text() === 'Connect SonicBoom')!.trigger('click');
  expect(wrapper.emitted('test')).toBeTruthy();
  await wrapper.setProps({ connection: { ok: false, reachable: true, latencyMs: 3, at: 3,
    serverInfo: { authRequired: true, authorized: false, modelStatus: 'loading', setupComplete: true, playback: true } } });
  await flushPromises();
  expect(document.querySelector('#provision-password')).not.toBeNull();
});

test('an unreachable server shows a simple recovery instruction before technical details', async () => {
  const wrapper = mountEditor(false, false);
  await wrapper.setProps({ connection: { ok: false, reachable: false, error: 'raw network failure', latencyMs: 5, at: 2 } });
  expect(wrapper.text()).toContain('Open SonicBoom on this computer');
  expect(wrapper.text()).not.toContain('raw network failure');
  await wrapper.findAll('button').find(button => button.text() === 'Advanced settings')!.trigger('click');
  expect(wrapper.text()).toContain('raw network failure');
});
