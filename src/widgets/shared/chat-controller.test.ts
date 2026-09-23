import { describe, expect, test } from 'bun:test';

import {
  ChatController,
  type ChatMessageView,
} from './chat-controller.ts';
import {
  makeTestChatEnvelope,
  makeTestGiftEnvelope,
} from './test-events.ts';

function setup(options: { limit?: number } = {}) {
  const snapshots: ChatMessageView[][] = [];
  const controller = new ChatController(
    {
      onMessages: (messages) => void snapshots.push(messages.map((message) => ({ ...message }))),
    },
    { limit: options.limit },
  );
  return { snapshots, controller };
}

describe('chat controller', () => {
  test('appends messages in arrival order', () => {
    const { snapshots, controller } = setup();
    expect(controller.handleEnvelope(makeTestChatEnvelope({ comment: 'first' }))).toBe('added');
    expect(controller.handleEnvelope(makeTestChatEnvelope({ comment: 'second' }))).toBe('added');
    expect(controller.snapshot.map((message) => message.text)).toEqual(['first', 'second']);
    expect(snapshots).toHaveLength(2);
    expect(snapshots[1]?.map((message) => message.text)).toEqual(['first', 'second']);
  });

  test('carries sender identity and avatar', () => {
    const { controller } = setup();
    controller.handleEnvelope(
      makeTestChatEnvelope({
        comment: 'hi',
        user: { nickname: 'Chatter', uniqueId: 'chatter', avatarUrl: 'https://cdn.example/c.png' },
      }),
    );
    expect(controller.snapshot[0]).toMatchObject({
      displayName: 'Chatter',
      uniqueId: '@chatter',
      avatarUrl: 'https://cdn.example/c.png',
      text: 'hi',
    });
  });

  test('caps the list by dropping the oldest messages', () => {
    const { controller } = setup({ limit: 3 });
    for (const text of ['one', 'two', 'three', 'four']) {
      controller.handleEnvelope(makeTestChatEnvelope({ comment: text }));
    }
    expect(controller.snapshot.map((message) => message.text)).toEqual(['two', 'three', 'four']);
  });

  test('ignores history, duplicates, empty, and non-chat events', () => {
    const { snapshots, controller } = setup();
    expect(controller.handleEnvelope(makeTestChatEnvelope({ isHistory: true }))).toBe('history');
    const envelope = makeTestChatEnvelope({ comment: 'kept' });
    expect(controller.handleEnvelope(envelope)).toBe('added');
    expect(controller.handleEnvelope(envelope)).toBe('duplicate');
    expect(controller.handleEnvelope(makeTestChatEnvelope({ comment: '   ' }))).toBe('ignored');
    expect(controller.handleEnvelope(makeTestGiftEnvelope())).toBe('ignored');
    expect(controller.snapshot.map((message) => message.text)).toEqual(['kept']);
    expect(snapshots).toHaveLength(1);
  });

  test('clear empties the overlay', () => {
    const { snapshots, controller } = setup();
    controller.handleEnvelope(makeTestChatEnvelope({ comment: 'kept' }));
    controller.clear();
    expect(controller.snapshot).toEqual([]);
    expect(snapshots[snapshots.length - 1]).toEqual([]);
  });
});
