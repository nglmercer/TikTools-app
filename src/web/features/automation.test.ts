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

describe('useAutomation fire event', () => {
  function fired(result: unknown): { client: ControlClient; calls: Array<{ method: string; params: unknown }> } {
    const { client } = fakeControl();
    const calls: Array<{ method: string; params: unknown }> = [];
    client.call = ((method: string, params: unknown) => {
      calls.push({ method, params });
      if (method === 'automation.fire') return Promise.resolve(result);
      return Promise.reject(new Error(`unexpected call ${method}`));
    }) as ControlClient['call'];
    return { client, calls };
  }

  async function flush(times = 4): Promise<void> {
    for (let i = 0; i < times; i += 1) {
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
  }

  const event = {
    schemaVersion: 1 as const,
    id: 'evt-1',
    name: 'Galaxy gate',
    enabled: true,
    trigger: 'tiktok.gift',
    filters: [],
    cooldownMs: 0,
    cooldownScope: 'user' as const,
    actionIds: [],
    runMode: 'all' as const,
  };

  test('fires the trigger and surfaces the outcome as a test-panel entry', async () => {
    const { client, calls } = fired({
      trigger: 'tiktok.gift',
      eventSource: 'live',
      matched: 2,
      status: 'ok',
      summary: 'Fired tiktok.gift: 2 events matched, actions executed.',
      durationMs: 3,
    });
    const automation = useAutomation(client);
    automation.handleFireEvent(event);
    await flush();
    expect(calls).toEqual([
      { method: 'automation.fire', params: { trigger: 'tiktok.gift', record: event } },
    ]);
    expect(automation.behaviorError.value).toBe('');
    expect(automation.behaviorTestRuns.value).toHaveLength(1);
    expect(automation.behaviorTestRuns.value[0]).toMatchObject({
      status: 'ok',
      actionName: 'Galaxy gate',
      summary: 'Fired tiktok.gift: 2 events matched, actions executed.',
      eventSource: 'live',
    });
  });

  test('zero-match fire surfaces as an error entry without touching the error bar', async () => {
    const { client } = fired({
      trigger: 'tiktok.gift',
      eventSource: 'sample',
      matched: 0,
      status: 'error',
      summary: 'Fired tiktok.gift: no enabled event matched.',
      durationMs: 1,
    });
    const automation = useAutomation(client);
    automation.handleFireEvent(event);
    await flush();
    expect(automation.behaviorError.value).toBe('');
    expect(automation.behaviorTestRuns.value[0]).toMatchObject({
      status: 'error',
      error: 'Fired tiktok.gift: no enabled event matched.',
    });
  });

  test('transport failure surfaces in the error bar', async () => {
    const { client } = fakeControl();
    const automation = useAutomation(client);
    automation.handleFireEvent(event);
    await flush();
    expect(automation.behaviorError.value).toBe('not stubbed');
    expect(automation.behaviorTestRuns.value).toEqual([]);
  });
});

describe('useAutomation hotkey access request', () => {
  function stubbed(result: unknown): {
    client: ControlClient;
    calls: string[];
  } {
    const { client } = fakeControl();
    const calls: string[] = [];
    client.call = ((method: string) => {
      calls.push(method);
      if (method === 'system.requestInputAccess') return Promise.resolve(result);
      if (method === 'automation.runs') return Promise.resolve({ runs: [] });
      return Promise.resolve({ actions: [], events: [], plugins: [], actionTypes: [], translations: {} });
    }) as ControlClient['call'];
    return { client, calls };
  }

  async function flush(times = 4): Promise<void> {
    for (let i = 0; i < times; i += 1) {
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
  }

  test('granted access refreshes and clears the pending flag', async () => {
    const { client, calls } = stubbed({ granted: true, message: 'granted' });
    const automation = useAutomation(client);
    automation.handleRequestHotkeyAccess();
    expect(automation.hotkeyAccessPending.value).toBe(true);
    await flush();
    expect(calls.filter((method) => method === 'system.requestInputAccess')).toHaveLength(1);
    expect(calls).toContain('automation.snapshot');
    expect(automation.hotkeyAccessPending.value).toBe(false);
    expect(automation.behaviorError.value).toBe('');
  });

  test('denied access surfaces the host message without refreshing', async () => {
    const { client, calls } = stubbed({ granted: false, message: 'dismissed' });
    const automation = useAutomation(client);
    automation.handleRequestHotkeyAccess();
    await flush();
    expect(automation.hotkeyAccessPending.value).toBe(false);
    expect(automation.behaviorError.value).toBe('dismissed');
    expect(calls).not.toContain('automation.snapshot');
  });

  test('double clicks issue a single request', async () => {
    const { client, calls } = stubbed({ granted: true, message: 'granted' });
    const automation = useAutomation(client);
    automation.handleRequestHotkeyAccess();
    automation.handleRequestHotkeyAccess();
    await flush();
    expect(calls.filter((method) => method === 'system.requestInputAccess')).toHaveLength(1);
  });
});
