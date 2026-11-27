import { expect, test } from 'vitest';
import type { ControlClient, PushHandler } from '@/shared/api/control-client.ts';
import { usePlugins } from '@/entities/plugin/model/plugins-state.ts';
import { useAutomation } from '../../../../../../src/web/entities/automation/model/automation-state.ts';
import { RPC, TOPIC } from '../../../../../shared/index.ts';

type TopicSink = (data: never) => void;

function harness() {
  const topics = new Map<string, Set<TopicSink>>();
  const pushes = new Map<string, Set<PushHandler>>();
  const calls: string[] = [];
  let finish!: (value: unknown) => void;
  const stalled = new Promise<unknown>((resolve) => { finish = resolve; });
  const awaitStalled: Function = () => stalled;
  const client: ControlClient = {
    attach() {}, detach() {},
    async call<T>(method: string): Promise<T> {
      calls.push(method);
      return Reflect.apply(awaitStalled, undefined, []);
    },
    onTopic(topic, handler) {
      const list = topics.get(topic) ?? new Set<TopicSink>();
      topics.set(topic, list);
      list.add(handler);
      return () => { list.delete(handler); };
    },
    onPush(type, handler) {
      const list = pushes.get(type) ?? new Set<PushHandler>();
      pushes.set(type, list);
      list.add(handler);
      return () => { list.delete(handler); };
    },
    onTransportError: () => () => {}, onGap: () => () => {},
  };
  const listeners = () => [...topics.values(), ...pushes.values()].reduce((total, list) => total + list.size, 0);
  const emit = (topic: string, data: unknown) => {
    for (const handler of topics.get(topic) ?? []) Reflect.apply(handler, undefined, [data]);
  };
  return { client, topics, calls, emit, finish: (value: unknown) => finish(value), listeners };
}

test('100 feature remounts return subscriptions to baseline and cancel debounced refresh', async () => {
  const state = harness();
  for (let cycle = 0; cycle < 100; ++cycle) {
    const automation = useAutomation(state.client);
    const plugins = usePlugins(state.client, { translate: (key) => key, refreshBehavior: async () => {}, reportError() {} });
    state.emit(TOPIC.workflowChanged, {});
    state.emit('plugin.progress', { pluginId: 'p', state: 'ready', message: '' });
    automation.dispose();
    plugins.dispose();
    automation.dispose();
    plugins.dispose();
    expect(state.listeners()).toBe(0);
  }
  await new Promise((resolve) => setTimeout(resolve, 250));
  expect(state.calls).toHaveLength(0);
});

test('disposed plugins ignore outstanding settings completion', async () => {
  const state = harness();
  const plugins = usePlugins(state.client, { translate: (key) => key, refreshBehavior: async () => {}, reportError() {} });
  plugins.handleGetPluginSettings('p');
  plugins.dispose();
  state.finish({ pluginId: 'p', schema: {}, values: { old: true } });
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(plugins.pluginSettings.value).toEqual({});
  expect(state.calls).toEqual([RPC.pluginsSettingsGet]);
});
