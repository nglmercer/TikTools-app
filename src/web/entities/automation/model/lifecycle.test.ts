import { expect, test } from 'vitest';
import type { ControlClient, PushHandler, TopicHandler } from '@/shared/api/control-client.ts';
import { usePlugins } from '@/entities/plugin/model/plugins-state.ts';
import { useAutomation } from './automation-state.ts';

function harness() {
  const topics = new Map<string, Set<TopicHandler>>();
  const pushes = new Map<string, Set<PushHandler>>();
  const calls: string[] = [];
  let finish!: (value: unknown) => void;
  const stalled = new Promise<unknown>((resolve) => { finish = resolve; });
  const client: ControlClient = {
    attach() {}, detach() {},
    call<T>(method: string) { calls.push(method); return stalled as Promise<T>; },
    onTopic<T>(topic: string, handler: (data: T) => void) {
      const list = topics.get(topic) ?? new Set<TopicHandler>();
      topics.set(topic, list); list.add(handler as TopicHandler);
      return () => { list.delete(handler as TopicHandler); };
    },
    onPush(type, handler) {
      const list = pushes.get(type) ?? new Set<PushHandler>();
      pushes.set(type, list); list.add(handler);
      return () => { list.delete(handler); };
    },
    onTransportError: () => () => {}, onGap: () => () => {},
  };
  const listeners = () => [...topics.values(), ...pushes.values()].reduce((total, list) => total + list.size, 0);
  return { client, topics, calls, finish: (value: unknown) => finish(value), listeners };
}

test('100 feature remounts return subscriptions to baseline and cancel debounced refresh', async () => {
  const state = harness();
  for (let cycle = 0; cycle < 100; ++cycle) {
    const automation = useAutomation(state.client);
    const plugins = usePlugins(state.client, { translate: (key) => key, refreshBehavior: async () => {}, reportError() {} });
    for (const handler of state.topics.get('workflow.changed') ?? []) handler({});
    for (const handler of state.topics.get('plugin.progress') ?? []) handler({ pluginId: 'p', state: 'ready', message: '' });
    automation.dispose(); plugins.dispose();
    automation.dispose(); plugins.dispose();
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
  expect(state.calls).toEqual(['plugins.settings.get']);
});
