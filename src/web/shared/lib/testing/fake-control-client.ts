import { createPinia, setActivePinia } from 'pinia';
import { createApp, type Plugin } from 'vue';
import type { ControlCallOptions, ControlClient, RpcParams } from '@/shared/api/control-client.ts';
import { controlClientKey } from '@/shared/api/use-control-client.ts';

export interface RecordedCall {
  method: string;
  params: RpcParams | undefined;
}

export function createFakeControlClient(handler: (method: string, params: RpcParams | undefined, options?: ControlCallOptions) => unknown) {
  const calls: RecordedCall[] = [];
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: (async (method: string, params?: RpcParams, options?: ControlCallOptions) => {
      calls.push({ method, params });
      return handler(method, params, options);
    }) as ControlClient['call'],
    onTopic: () => () => {},
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  return { client, calls };
}

export function storePlugins(client: ControlClient): Plugin[] {
  return [createPinia(), { install: (app) => void app.provide(controlClientKey, client) }];
}

export function createStoreHarness(client: ControlClient) {
  const pinia = createPinia();
  const host = createApp({ render: () => null });
  host.use(pinia).provide(controlClientKey, client);
  setActivePinia(pinia);
  const plugins: Plugin[] = [pinia, { install: (app) => void app.provide(controlClientKey, client) }];
  return { plugins, use: <T>(factory: () => T): T => host.runWithContext(factory) };
}
