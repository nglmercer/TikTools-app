import { describe, expect, test } from 'bun:test';

import type { ControlClient, RpcParams } from '../platform/control-client.ts';
import type { ViewerRecord } from '../types.ts';
import { defaultPointsConfig, usePoints } from './points.ts';

function viewer(uniqueId: string, points: number): ViewerRecord {
  return {
    uniqueId,
    points,
    level: 1,
    isSubscriber: false,
    totalChats: 0,
    totalCoins: 0,
    totalLikes: 0,
    totalShares: 0,
    firstSeen: 0,
    lastSeen: 0,
  };
}

function stubControl(board: ViewerRecord[]): ControlClient & {
  calls: string[];
  topics: Map<string, (data: never) => void>;
} {
  const calls: string[] = [];
  const topics = new Map<string, (data: never) => void>();
  return {
    calls,
    topics,
    attach: () => {},
    detach: () => {},
    call: <T,>(method: string, _params?: RpcParams): Promise<T> => {
      void _params;
      calls.push(method);
      if (method === 'points.leaderboard') return Promise.resolve({ viewers: board } as T);
      if (method === 'points.config.get') return Promise.resolve(defaultPointsConfig as T);
      return Promise.reject(new Error(`unstubbed ${method}`)) as Promise<T>;
    },
    onTopic: <T,>(topic: string, handler: (data: T) => void): (() => void) => {
      topics.set(topic, handler as (data: never) => void);
      return () => {};
    },
    onPush: () => () => {},
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
}

async function flushPromises(): Promise<void> {
  for (let i = 0; i < 10; i += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
}

describe('usePoints new viewers', () => {
  test('awards for unknown viewers refresh the board so they appear', async () => {
    const board = [viewer('new-user', 5)];
    const control = stubControl(board);
    const points = usePoints(control);
    expect(points.leaderboard.value).toHaveLength(0);

    const handler = control.topics.get('points.changed');
    expect(handler).toBeDefined();
    handler?.({ uniqueId: 'new-user', delta: 5, totalPoints: 5, level: 1 } as never);
    await flushPromises();

    expect(control.calls).toContain('points.leaderboard');
    expect(points.leaderboard.value.map((entry) => entry.uniqueId)).toContain('new-user');
  });

  test('awards for known viewers patch locally without a refresh', async () => {
    const control = stubControl([viewer('alice', 10)]);
    const points = usePoints(control);
    await points.refresh();
    const callsBefore = control.calls.length;

    const handler = control.topics.get('points.changed');
    handler?.({ uniqueId: 'alice', delta: 2, totalPoints: 12, level: 1 } as never);
    await flushPromises();

    expect(control.calls).toHaveLength(callsBefore);
    expect(points.leaderboard.value[0]?.points).toBe(12);
  });

  test('a burst of unknown viewers triggers only one refresh', async () => {
    const control = stubControl([viewer('a', 1), viewer('b', 1)]);
    const points = usePoints(control);
    const handler = control.topics.get('points.changed');
    expect(handler).toBeDefined();

    handler?.({ uniqueId: 'a', delta: 1, totalPoints: 1, level: 1 } as never);
    handler?.({ uniqueId: 'b', delta: 1, totalPoints: 1, level: 1 } as never);
    handler?.({ uniqueId: 'c', delta: 1, totalPoints: 1, level: 1 } as never);
    await flushPromises();

    expect(control.calls.filter((method) => method === 'points.leaderboard')).toHaveLength(1);
    expect(points.leaderboard.value).toHaveLength(2);
  });
});
