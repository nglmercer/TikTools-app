import { expect, test } from 'vitest';
import { usePlugins } from './plugins-state.ts';
import type { ControlClient, RpcParams } from '@/shared/api/control-client.ts';

function harness(result: Promise<unknown>) {
  const calls: Array<{method: string; params?: RpcParams}> = [];
  const client: ControlClient = {
    attach() {}, detach() {},
    call<T>(method: string, params?: RpcParams) {
      calls.push({ method, params });
      if (method === 'plugins.token.import') return result as Promise<T>;
      return Promise.resolve(method === 'plugins.settings.get'
        ? { pluginId: 'repo.game', schema: {}, values: { apiToken: '••••••••' } }
        : { pluginId: 'repo.game', ok: true, latencyMs: 1 }) as Promise<T>;
    },
    onTopic: () => () => {}, onPush: () => () => {},
    onTransportError: () => () => {}, onGap: () => () => {},
  };
  const plugins = usePlugins(client, { translate: (key) => key, refreshBehavior: async () => {}, reportError() {} });
  return { plugins, calls };
}
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

test('token import sends only plugin ID and refreshes masked settings and connection', async () => {
  const { plugins, calls } = harness(Promise.resolve({ pluginId: 'repo.game', ok: true }));
  plugins.handleImportPluginToken('repo.game');
  plugins.handleImportPluginToken('repo.game');
  expect(plugins.pluginProvision.value['repo.game']?.working).toBe(true);
  await flush();
  expect(calls).toEqual([
    { method: 'plugins.token.import', params: { pluginId: 'repo.game' } },
    { method: 'plugins.settings.get', params: { pluginId: 'repo.game' } },
    { method: 'plugins.health', params: { pluginId: 'repo.game' } },
  ]);
  expect(plugins.pluginProvision.value['repo.game']?.ok).toBe(true);
  expect(plugins.pluginSettings.value['repo.game']?.values.apiToken).toBe('••••••••');
  plugins.dispose();
});

test('cancelled import stays quiet and does not re-probe or overwrite settings', async () => {
  const { plugins, calls } = harness(Promise.resolve({ pluginId: 'repo.game', ok: false, error: null }));
  plugins.handleImportPluginToken('repo.game');
  await flush();
  expect(calls).toHaveLength(1);
  expect(plugins.pluginProvision.value['repo.game']).toEqual({ working: false, ok: false, message: '' });
  expect(plugins.pluginSettings.value).toEqual({});
  plugins.dispose();
});

test('failed imports display safe errors and disposed requests cannot update UI', async () => {
  const state = harness(Promise.resolve({ pluginId: 'repo.game', ok: false, error: 'Token entry was not found.' }));
  state.plugins.handleImportPluginToken('repo.game');
  await flush();
  expect(state.plugins.pluginProvision.value['repo.game']?.message).toBe('Token entry was not found.');
  expect(state.calls).toHaveLength(1);
  state.plugins.dispose();
  let finish!: (value: unknown) => void;
  const disposed = harness(new Promise((resolve) => { finish = resolve; }));
  disposed.plugins.handleImportPluginToken('repo.game');
  disposed.plugins.dispose();
  finish({ pluginId: 'repo.game', ok: true });
  await flush();
  expect(disposed.calls).toHaveLength(1);
  expect(disposed.plugins.pluginProvision.value['repo.game']?.working).toBe(true);
});

test('automatic setup requires a source selection before import and never sends filesystem paths', async () => {
  const candidates = [{ id: 'aabbccddeeff0011', label: 'Steam — REPO' }, { id: '1122334455667788', label: 'Mod profile: Stream' }];
  const { plugins, calls } = harness(Promise.resolve({ pluginId: 'repo.game', ok: false, candidates }));
  plugins.handleImportPluginToken('repo.game', { mode: 'auto' });
  await flush();
  expect(calls).toEqual([{ method: 'plugins.token.import', params: { pluginId: 'repo.game', mode: 'auto' } }]);
  expect(plugins.pluginProvision.value['repo.game']?.candidates).toEqual(candidates);
  plugins.handleImportPluginToken('repo.game', { mode: 'candidate', candidateId: '1122334455667788' });
  await flush();
  expect(calls[1]?.params).toEqual({ pluginId: 'repo.game', mode: 'candidate', candidateId: '1122334455667788' });
  expect(plugins.pluginSettings.value).toEqual({});
  plugins.dispose();
});

test('folder setup invokes the host picker and refreshes connection after importing', async () => {
  const { plugins, calls } = harness(Promise.resolve({ pluginId: 'repo.game', ok: true }));
  plugins.handleImportPluginToken('repo.game', { mode: 'folder' });
  await flush();
  expect(calls[0]).toEqual({ method: 'plugins.token.import', params: { pluginId: 'repo.game', mode: 'folder' } });
  expect(calls.map((call) => call.method)).toEqual(['plugins.token.import', 'plugins.settings.get', 'plugins.health']);
  plugins.dispose();
});
