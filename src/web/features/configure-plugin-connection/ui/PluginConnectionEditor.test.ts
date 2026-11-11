import { enableAutoUnmount, flushPromises } from '@vue/test-utils';
import { afterEach, describe, expect, test, vi } from 'vitest';
import { AUTOSAVE_DEBOUNCE_MS } from '@/entities/plugin/lib/connection-settings.ts';
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
    pluginId: 'sonicboom.server', pluginName: 'SonicBoom', guided: true, actionOptions: {}, settings,
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

test('the sign-in dialog outlives the authorization button when the server becomes authorized', async () => {
  const wrapper = mountEditor(true, false);
  await wrapper.findAll('button').find(button => button.text() === 'Connect SonicBoom')!.trigger('click');
  await wrapper.setProps({ connection: { ...wrapper.props('connection')!, at: 2 } });
  await flushPromises();
  expect(document.querySelector('#provision-password')).not.toBeNull();
  await wrapper.setProps({ connection: { ok: true, reachable: true, latencyMs: 4, at: 3,
    serverInfo: { authRequired: true, authorized: true, modelStatus: 'ready', setupComplete: true, playback: true } } });
  await flushPromises();
  expect(wrapper.findAll('button').some(button => button.text() === 'Authorize')).toBe(false);
  expect(wrapper.findAll('dialog')).toHaveLength(1);
  expect(document.querySelector('#provision-password')).not.toBeNull();
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

test('the sectioned layout keeps connection fields and gameplay defaults side by side', () => {
  const wrapper = mountWithPlugins(PluginConnectionEditor, { props: {
    pluginId: 'peak.game', pluginName: 'PEAK', actionOptions: {}, layout: 'sections',
    settings: {
      schema: { type: 'object', properties: {
        serverUrl: { type: 'string', format: 'uri', title: 'Server URL', default: 'http://127.0.0.1:17454' },
        apiToken: { type: 'string', title: 'API token', secret: true },
        quantity: { type: 'integer', title: 'Default quantity', default: 1, minimum: 1, maximum: 5 },
        playerId: { type: 'string', title: 'Default player ID', default: '' },
      } },
      values: { serverUrl: 'http://127.0.0.1:17454', apiToken: '••••••••', quantity: 1, playerId: '' },
    },
    connection: { ok: false, latencyMs: 0, at: 1 },
  } });
  const sections = wrapper.findAll('section.base-section');
  expect(sections.map(section => section.get('.base-section__title').text())).toEqual(['Connection', 'Defaults']);
  expect(sections[0]!.find('input[type="password"]').exists()).toBe(true);
  expect(sections[0]!.find('input[role="spinbutton"]').exists()).toBe(false);
  expect(sections[1]!.find('input[role="spinbutton"]').exists()).toBe(true);
});

test('an unreachable server keeps recovery guidance and moves technical details to logs', async () => {
  const wrapper = mountEditor(false, false);
  await wrapper.setProps({ connection: { ok: false, reachable: false, error: 'raw network failure', latencyMs: 5, at: 2 } });
  expect(wrapper.text()).toContain('Open SonicBoom on this computer');
  expect(wrapper.text()).not.toContain('raw network failure');
  await wrapper.findAll('button').find(button => button.text() === 'Advanced settings')!.trigger('click');
  expect(wrapper.text()).not.toContain('raw network failure');
  expect(wrapper.find('button[aria-label="Connection failed. View logs"]').exists()).toBe(true);
});

describe.each(['stacked', 'sections'] as const)('credentials and authorization in the %s layout', (layout) => {
  const settings = {
    schema: { type: 'object', properties: {
      serverUrl: { type: 'string', format: 'uri', title: 'Server URL', default: 'http://127.0.0.1:17454' },
      apiToken: { type: 'string', title: 'API token', secret: true },
      playerId: { type: 'string', title: 'Default player ID', default: '' },
    } },
    values: { serverUrl: 'http://127.0.0.1:17454', apiToken: '••••••••', playerId: 'p1' },
  };
  function mountCredentials(extra: Record<string, unknown> = {}) {
    return mountWithPlugins(PluginConnectionEditor, { attachTo: document.body, props: {
      pluginId: 'peak.game', pluginName: 'PEAK', actionOptions: {}, layout, settings,
      supportsTokenImport: true, supportsTokenDiscovery: true, supportsProvisioning: true,
      connection: { ok: false, latencyMs: 0, at: 1 },
      provisionState: { working: false, ok: false, message: 'Pick one', candidates: [{ id: 'c1', label: 'Config A' }, { id: 'c2', label: 'Config B' }] },
      ...extra,
    } });
  }

  test('automatic discovery, folder, file and candidate selection report their mode and nothing else', async () => {
    const wrapper = mountCredentials();
    const buttons = wrapper.findAll('button');
    const [auto, folder] = buttons.filter(button => button.attributes('title')?.startsWith('Finds installed games'));
    expect(auto).toBeDefined();
    expect(folder).toBeDefined();
    await auto!.trigger('click');
    await folder!.trigger('click');
    await buttons.find(button => button.attributes('title')?.startsWith('Select the game or server configuration file'))!.trigger('click');
    await buttons.filter(button => button.text() === 'Use this configuration')[1]!.trigger('click');
    expect(wrapper.emitted('importToken')).toEqual([
      ['peak.game', { mode: 'auto' }],
      ['peak.game', { mode: 'folder' }],
      ['peak.game', undefined],
      ['peak.game', { mode: 'candidate', candidateId: 'c2' }],
    ]);
    expect(wrapper.text()).toContain('Config A');
    expect(wrapper.text()).toContain('Pick one');
    expect(wrapper.emitted('test')).toBeUndefined();
    expect(wrapper.emitted('loadSettings')).toBeUndefined();
    expect(wrapper.emitted('loadOptions')).toBeUndefined();
  });

  test('every credential action is disabled while an import is working', () => {
    const wrapper = mountCredentials({ provisionState: { working: true, ok: false, message: '', candidates: [{ id: 'c1', label: 'Config A' }] } });
    const actions = wrapper.findAll('button').filter(button =>
      /^(Finds installed games|Select the game or server)/.test(button.attributes('title') ?? '')
      || button.text() === 'Use this configuration' || button.text() === 'Get API token…');
    expect(actions.length).toBeGreaterThanOrEqual(4);
    for (const button of actions) expect(button.attributes('disabled')).toBeDefined();
  });

  test('the sign-in dialog is mounted once and opens from the authorization button', async () => {
    const wrapper = mountCredentials();
    expect(wrapper.findAllComponents({ name: 'ProvisionTokenDialog' })).toHaveLength(1);
    expect(wrapper.findAll('dialog')).toHaveLength(1);
    expect(document.querySelectorAll('#provision-user')).toHaveLength(0);
    await wrapper.findAll('button').find(button => button.text() === 'Get API token…')!.trigger('click');
    expect(document.querySelectorAll('#provision-user')).toHaveLength(1);
    expect(wrapper.findAll('dialog')).toHaveLength(1);
    expect(wrapper.findAllComponents({ name: 'ProvisionTokenDialog' })).toHaveLength(1);
  });

  test('one edit produces one save sequence, and the other form keeps its fields', async () => {
    vi.useFakeTimers();
    try {
      const wrapper = mountCredentials({ provisionState: undefined });
      const url = wrapper.find<HTMLInputElement>('input[type="text"], input[type="url"], input:not([type])');
      expect(url.exists()).toBe(true);
      await url.setValue('http://127.0.0.1:9000');
      await url.setValue('http://127.0.0.1:9001');
      vi.advanceTimersByTime(AUTOSAVE_DEBOUNCE_MS - 1);
      expect(wrapper.emitted('save')).toBeUndefined();
      vi.advanceTimersByTime(1);
      const saves = wrapper.emitted('save')!;
      expect(saves).toHaveLength(1);
      expect(saves[0]![1]).toMatchObject({ serverUrl: 'http://127.0.0.1:9001', playerId: 'p1' });
      vi.advanceTimersByTime(AUTOSAVE_DEBOUNCE_MS * 5);
      expect(wrapper.emitted('save')).toHaveLength(1);
      expect(wrapper.emitted('test')).toBeUndefined();
    } finally {
      vi.useRealTimers();
    }
  });
});

test('editing the connection form and the defaults form saves one shared document', async () => {
  vi.useFakeTimers();
  try {
    const wrapper = mountWithPlugins(PluginConnectionEditor, { props: {
      pluginId: 'peak.game', pluginName: 'PEAK', actionOptions: {}, layout: 'sections',
      settings: { schema: { type: 'object', properties: {
        serverUrl: { type: 'string', format: 'uri', title: 'Server URL', default: 'http://127.0.0.1:17454' },
        playerId: { type: 'string', title: 'Default player ID', default: '' },
      } }, values: { serverUrl: 'http://127.0.0.1:17454', playerId: '' } },
      connection: { ok: false, latencyMs: 0, at: 1 },
    } });
    const [connectionSection, defaultsSection] = wrapper.findAll('section.base-section');
    await connectionSection!.get('input').setValue('http://127.0.0.1:9000');
    await defaultsSection!.get('input').setValue('player-7');
    vi.advanceTimersByTime(AUTOSAVE_DEBOUNCE_MS);
    const saves = wrapper.emitted('save')!;
    expect(saves).toHaveLength(1);
    expect(saves[0]![1]).toEqual({ serverUrl: 'http://127.0.0.1:9000', playerId: 'player-7' });
    expect((connectionSection!.get('input').element as HTMLInputElement).value).toBe('http://127.0.0.1:9000');
  } finally {
    vi.useRealTimers();
  }
});
