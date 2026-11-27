import { testMessage } from '@/shared/i18n/test-message.ts';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { createLiveFeedState } from '../live-feed-state.ts';
import type { ControlClient, RpcParams } from '@/shared/api/control-client.ts';

export { testMessage, beforeEach, describe, expect, it, vi, createLiveFeedState, type ControlClient };

export const t = (key: string): string => testMessage('en', key);

export function callbacks(systemAuthor: string) {
  return { systemAuthor: () => systemAuthor, discontinuityText: () => t('feed.resumed') };
}

export type TopicHandler = (data: never) => void;

type Responder = (method: string, params: RpcParams | undefined) => unknown;

/** A full `ControlClient` fake: topic handlers are recorded per topic and `emit` replays a payload to them. */
function createBus(initialResponder: Responder) {
  const handlers = new Map<string, TopicHandler[]>();
  const unsubscribed: string[] = [];
  let responder = initialResponder;
  const control: ControlClient = {
    attach: () => {},
    detach: () => {},
    // Function.prototype.call is typed as returning any, which lets a fake answer any `call<T>` without asserting T.
    call: async (method: string, params?: RpcParams) => Reflect.apply(Function.prototype.call, responder, [undefined, method, params]),
    onTopic: <T>(topic: string, handler: (data: T) => void) => {
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
    onTransportError: () => () => {},
    onGap: () => () => {},
  };
  return {
    handlers,
    unsubscribed,
    control,
    respond(next: Responder): void {
      responder = next;
    },
    emit(topic: string, data: unknown): void {
      for (const handler of handlers.get(topic) ?? []) Reflect.apply(handler, undefined, [data]);
    },
  };
}

function noResponder(method: string): never {
  throw new Error(`createControl has no responder for ${method}`);
}

export function createControl() {
  return createBus(noResponder);
}

export const chatEvent = (prefix: string, author: string, index: number) => ({
  event: {
    kind: 'chat' as const,
    author: `${prefix}-${author}`,
    text: `${prefix} message ${index}`,
  },
});

export function createBackfillControl(responder: (params: RpcParams | undefined) => unknown) {
  const calls: Array<{ method: string; params: unknown }> = [];
  const bus = createBus((method, params) => {
    calls.push({ method, params });
    return responder(params);
  });
  return { handlers: bus.handlers, calls, control: bus.control, emit: bus.emit };
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
