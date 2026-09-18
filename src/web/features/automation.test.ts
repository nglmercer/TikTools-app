import { describe, expect, test } from 'bun:test';

import type { HostMessage } from '../../shared/messages.ts';
import type { ControlClient, PushHandler, TopicHandler } from '../platform/control-client.ts';
import { useAutomation } from './automation.ts';

function fakeControl(): {
  client: ControlClient;
  pushes: Map<string, PushHandler[]>;
  topics: Map<string, TopicHandler[]>;
} {
  const pushes = new Map<string, PushHandler[]>();
  const topics = new Map<string, TopicHandler[]>();
  const client: ControlClient = {
    attach: () => {},
    detach: () => {},
    call: () => Promise.reject(new Error('not stubbed')),
    onTopic: (topic, handler) => {
      const list = topics.get(topic) ?? [];
      list.push(handler as TopicHandler);
      topics.set(topic, list);
      return () => {};
    },
    onPush: (type, handler) => {
      const list = pushes.get(type) ?? [];
      list.push(handler);
      pushes.set(type, list);
      return () => {};
    },
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  return { client, pushes, topics };
}

function emit<T>(handlers: T[] | undefined, value: T extends (arg: infer A) => void ? A : never): void {
  for (const handler of handlers ?? []) (handler as (arg: unknown) => void)(value);
}

describe('useAutomation live updates', () => {
  test('legacy behavior-runs push updates runs without refresh', () => {
    const { client, pushes } = fakeControl();
    const automation = useAutomation(client);
    expect(automation.behaviorRuns.value).toEqual([]);
    const runs = [
      {
        id: 'run-1',
        at: 1,
        status: 'ok' as const,
        actionName: 'Say',
        summary: 'hello',
        durationMs: 1,
        test: false,
        logs: [],
      },
    ];
    emit(pushes.get('behavior-runs'), { type: 'behavior-runs', runs } satisfies HostMessage);
    expect(automation.behaviorRuns.value).toEqual(runs);
  });

  test('automation.runs.changed topic updates runs without refresh', () => {
    const { client, topics } = fakeControl();
    const automation = useAutomation(client);
    const runs = [
      {
        id: 'run-2',
        at: 2,
        status: 'ok' as const,
        actionName: 'Say',
        summary: 'hi',
        durationMs: 1,
        test: false,
        logs: [],
      },
    ];
    emit(topics.get('automation.runs.changed'), { runs });
    expect(automation.behaviorRuns.value).toEqual(runs);
  });

  test('plugin.event hotkey press records the last event', () => {
    const { client, topics } = fakeControl();
    const automation = useAutomation(client);
    expect(automation.lastHotkeyEvent.value).toBeNull();
    emit(topics.get('plugin.event'), {
      pluginId: 'hotkeys',
      eventType: 'hotkey.pressed',
      event: { type: 'hotkey.pressed', data: { key: 'k', modifiers: 'ctrl', backend: 'rdev' } },
    });
    expect(automation.lastHotkeyEvent.value).toMatchObject({
      key: 'k',
      modifiers: 'ctrl',
      backend: 'rdev',
    });
    // Other plugin events do not touch the hotkey record.
    emit(topics.get('plugin.event'), {
      pluginId: 'other',
      eventType: 'other.thing',
      event: { type: 'other.thing', data: {} },
    });
    expect(automation.lastHotkeyEvent.value).toMatchObject({ key: 'k' });
  });

  test('plugin.status topic refreshes hotkey status', () => {
    const { client, topics } = fakeControl();
    const automation = useAutomation(client);
    const status = { platform: 'windows', session: 'n/a', backends: [] };
    emit(topics.get('plugin.status'), { status });
    expect(automation.hotkeyStatus.value).toEqual(status);
  });
});
