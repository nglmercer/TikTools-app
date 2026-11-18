import { testMessage } from '@/shared/i18n/test-message.ts';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createLiveFeedState } from '../live-feed-state.ts';
import type { ControlClient } from '@/shared/api/control-client.ts';

export { testMessage, beforeEach, describe, expect, it, vi, createLiveFeedState, type ControlClient };

export const t = (key: string): string => testMessage('en', key);

export function callbacks(systemAuthor: string) {
  return { systemAuthor: () => systemAuthor, discontinuityText: () => t('feed.resumed') };
}

export const frameQueue: Array<() => void> = [];
let frameId = 0;

export function installFrameControl(): void {
  frameQueue.length = 0;
  (globalThis as Record<string, unknown>).requestAnimationFrame = (callback: () => void) => {
    frameQueue.push(callback);
    frameId += 1;
    return frameId;
  };
  (globalThis as Record<string, unknown>).cancelAnimationFrame = () => {};
}

export function uninstallFrameControl(): void {
  frameQueue.length = 0;
  delete (globalThis as Record<string, unknown>).requestAnimationFrame;
  delete (globalThis as Record<string, unknown>).cancelAnimationFrame;
}

export function runFrames(count = 10): void {
  for (let i = 0; i < count && frameQueue.length > 0; i += 1) {
    const pending = frameQueue.splice(0, frameQueue.length);
    for (const callback of pending) callback();
  }
}

export type TopicHandler = (data: never) => void;

export function createControl() {
  const handlers = new Map<string, TopicHandler[]>();
  const unsubscribed: string[] = [];
  return {
    handlers,
    unsubscribed,
    control: {
      onTopic: (topic: string, handler: TopicHandler) => {
        const list = handlers.get(topic) ?? [];
        list.push(handler);
        handlers.set(topic, list);
        return () => {
          unsubscribed.push(topic);
          handlers.set(
            topic,
            (handlers.get(topic) ?? []).filter((entry) => entry !== handler),
          );
        };
      },
      onPush: () => () => {},
      onGap: () => () => {},
    } as never as ControlClient,
    emit(topic: string, data: never): void {
      for (const handler of handlers.get(topic) ?? []) handler(data);
    },
  };
}

export const chatEvent = (prefix: string, author: string, index: number) => ({
  event: {
    kind: 'chat' as const,
    author: `${prefix}-${author}`,
    text: `${prefix} message ${index}`,
  },
});

export function createBackfillControl(responder: (params: { after: number }) => unknown) {
  const handlers = new Map<string, TopicHandler[]>();
  const calls: Array<{ method: string; params: unknown }> = [];
  return {
    handlers,
    calls,
    control: {
      onTopic: (topic: string, handler: TopicHandler) => {
        const list = handlers.get(topic) ?? [];
        list.push(handler);
        handlers.set(topic, list);
        return () => {
          handlers.set(
            topic,
            (handlers.get(topic) ?? []).filter((entry) => entry !== handler),
          );
        };
      },
      onPush: () => () => {},
      onGap: () => () => {},
      call: async (method: string, params: unknown) => {
        calls.push({ method, params });
        return responder(params as { after: number });
      },
    } as never as ControlClient,
    emit(topic: string, data: never): void {
      for (const handler of handlers.get(topic) ?? []) handler(data);
    },
  };
}

export const likeEvent = (
  author: string,
  count: number,
  total: number,
  msgId: string,
  roomId = 'same-room',
  withIdentity = true,
) => ({
  event: {
    kind: 'like' as const,
    author,
    text: `sent ${count} likes`,
    likeCount: count,
    likeUserTotal: total,
    ...(withIdentity ? { likeViewerKey: author } : {}),
    creatorId: 'creator',
    roomId,
    msgId,
    i18nKey: 'likeSent',
    i18nParams: { count },
  },
});
