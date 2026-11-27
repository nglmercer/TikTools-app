import { HOST_MESSAGES } from '@/shared/testing/host-messages.ts';
import { expect, test } from 'vitest';
import { usePlugins } from '../../../../../../src/web/entities/plugin/model/plugins-state.ts';
import type { ControlClient, RpcParams } from '@/shared/api/control-client.ts';
import { PLUGIN_ID, RPC } from '../../../../../shared/index.ts';

function harness(result: Promise<unknown>) {
  const calls: Array<{
    method: string;
    params?: RpcParams;
  }> = [];
  const client: ControlClient = {
    attach() {}, detach() {},
    call<T>(method: string, params?: RpcParams) {
      calls.push({ method, params });
      if (method === RPC.pluginsTokenImport) return result as Promise<T>;
      return Promise.resolve(method === RPC.pluginsSettingsGet
        ? { pluginId: PLUGIN_ID.repoGame, schema: {}, values: { apiToken: '••••••••' } }
        : { pluginId: PLUGIN_ID.repoGame, ok: true, latencyMs: 1 }) as Promise<T>;
    },
    onTopic: () => () => {}, onPush: () => () => {},
    onTransportError: () => () => {}, onGap: () => () => {},
  };
  const plugins = usePlugins(client, { translate: (key) => key, refreshBehavior: async () => {}, reportError() {} });
  return { plugins, calls };
}
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

test('token import sends only plugin ID and refreshes masked settings and connection', async () => {
  const { plugins, calls } = harness(Promise.resolve({ pluginId: PLUGIN_ID.repoGame, ok: true }));
  plugins.handleImportPluginToken(PLUGIN_ID.repoGame);
  plugins.handleImportPluginToken(PLUGIN_ID.repoGame);
  expect(plugins.pluginProvision.value[PLUGIN_ID.repoGame]?.working).toBe(true);
  await flush();
  expect(calls).toEqual([
    { method: RPC.pluginsTokenImport, params: { pluginId: PLUGIN_ID.repoGame } },
    { method: RPC.pluginsSettingsGet, params: { pluginId: PLUGIN_ID.repoGame } },
    { method: 'plugins.health', params: { pluginId: PLUGIN_ID.repoGame } },
  ]);
  expect(plugins.pluginProvision.value[PLUGIN_ID.repoGame]?.ok).toBe(true);
  expect(plugins.pluginSettings.value[PLUGIN_ID.repoGame]?.values.apiToken).toBe('••••••••');
  plugins.dispose();
});

test('cancelled import stays quiet and does not re-probe or overwrite settings', async () => {
  const { plugins, calls } = harness(Promise.resolve({ pluginId: PLUGIN_ID.repoGame, ok: false, error: null }));
  plugins.handleImportPluginToken(PLUGIN_ID.repoGame);
  await flush();
  expect(calls).toHaveLength(1);
  expect(plugins.pluginProvision.value[PLUGIN_ID.repoGame]).toEqual({ working: false, ok: false, message: '' });
  expect(plugins.pluginSettings.value).toEqual({});
  plugins.dispose();
});

test('failed imports display safe errors and disposed requests cannot update UI', async () => {
  const state = harness(Promise.resolve({ pluginId: PLUGIN_ID.repoGame, ok: false, error: HOST_MESSAGES.tokenNotFound }));
  state.plugins.handleImportPluginToken(PLUGIN_ID.repoGame);
  await flush();
  expect(state.plugins.pluginProvision.value[PLUGIN_ID.repoGame]).toMatchObject({ working: false, ok: false });
  expect(state.plugins.pluginProvision.value[PLUGIN_ID.repoGame]?.message).toBeTruthy();
  expect(state.calls).toHaveLength(1);
  state.plugins.dispose();
  let finish!: (value: unknown) => void;
  const disposed = harness(new Promise((resolve) => { finish = resolve; }));
  disposed.plugins.handleImportPluginToken(PLUGIN_ID.repoGame);
  disposed.plugins.dispose();
  finish({ pluginId: PLUGIN_ID.repoGame, ok: true });
  await flush();
  expect(disposed.calls).toHaveLength(1);
  expect(disposed.plugins.pluginProvision.value[PLUGIN_ID.repoGame]?.working).toBe(true);
});

test('automatic setup requires a source selection before import and never sends filesystem paths', async () => {
  const candidates = [{ id: 'aabbccddeeff0011', label: 'Steam — REPO' }, { id: '1122334455667788', label: 'Mod profile: Stream' }];
  const { plugins, calls } = harness(Promise.resolve({ pluginId: PLUGIN_ID.repoGame, ok: false, candidates }));
  plugins.handleImportPluginToken(PLUGIN_ID.repoGame, { mode: 'auto' });
  await flush();
  expect(calls).toEqual([{ method: RPC.pluginsTokenImport, params: { pluginId: PLUGIN_ID.repoGame, mode: 'auto' } }]);
  expect(plugins.pluginProvision.value[PLUGIN_ID.repoGame]?.candidates).toEqual(candidates);
  plugins.handleImportPluginToken(PLUGIN_ID.repoGame, { mode: 'candidate', candidateId: '1122334455667788' });
  await flush();
  expect(calls[1]?.params).toEqual({ pluginId: PLUGIN_ID.repoGame, mode: 'candidate', candidateId: '1122334455667788' });
  expect(plugins.pluginSettings.value).toEqual({});
  plugins.dispose();
});

test('folder setup invokes the host picker and refreshes connection after importing', async () => {
  const { plugins, calls } = harness(Promise.resolve({ pluginId: PLUGIN_ID.repoGame, ok: true }));
  plugins.handleImportPluginToken(PLUGIN_ID.repoGame, { mode: 'folder' });
  await flush();
  expect(calls[0]).toEqual({ method: RPC.pluginsTokenImport, params: { pluginId: PLUGIN_ID.repoGame, mode: 'folder' } });
  expect(calls.map((call) => call.method)).toEqual([RPC.pluginsTokenImport, RPC.pluginsSettingsGet, 'plugins.health']);
  plugins.dispose();
});
