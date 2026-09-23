/**
 * Chat overlay controller: validates routed chat events, drops historical,
 * duplicate, and empty deliveries, and keeps a bounded message list for
 * the overlay. Rendering-agnostic; the Vue component only observes the
 * snapshot the handler receives.
 */

import { DEFAULT_CHAT_MESSAGE_LIMIT, DEFAULT_RECENT_IDS_CAPACITY } from './config.ts';
import { RecentEventIds } from './alert-queue.ts';
import { routeLiveEvent } from './event-router.ts';
import { displayNameFor, handleFor } from './event-types.ts';

export interface ChatMessageView {
  id: string;
  displayName: string;
  uniqueId: string;
  avatarUrl: string | null;
  text: string;
}

export interface ChatControllerHandlers {
  onMessages: (messages: ChatMessageView[]) => void;
}

export interface ChatControllerOptions {
  limit?: number;
  recentCapacity?: number;
}

export type ChatHandleResult = 'added' | 'ignored' | 'history' | 'duplicate';

export class ChatController {
  private readonly handlers: ChatControllerHandlers;
  private readonly limit: number;
  private readonly recentIds: RecentEventIds;

  private messages: ChatMessageView[] = [];

  constructor(handlers: ChatControllerHandlers, options: ChatControllerOptions = {}) {
    this.handlers = handlers;
    this.limit = Math.max(1, Math.floor(options.limit ?? DEFAULT_CHAT_MESSAGE_LIMIT));
    this.recentIds = new RecentEventIds(options.recentCapacity ?? DEFAULT_RECENT_IDS_CAPACITY);
  }

  get snapshot(): ChatMessageView[] {
    return [...this.messages];
  }

  handleEnvelope(value: unknown): ChatHandleResult {
    const routed = routeLiveEvent(value);
    if (!routed || routed.kind !== 'chat') return 'ignored';
    const { event } = routed;
    if (event.data.isHistory) return 'history';
    if (!this.recentIds.add(event.id)) return 'duplicate';
    const text = event.data.comment.trim();
    if (!text) return 'ignored';
    this.messages = [
      ...this.messages,
      {
        id: event.id,
        displayName: displayNameFor(event.user),
        uniqueId: handleFor(event.user),
        avatarUrl: event.user?.avatarUrl ?? null,
        text,
      },
    ].slice(-this.limit);
    this.handlers.onMessages([...this.messages]);
    return 'added';
  }

  clear(): void {
    this.messages = [];
    this.handlers.onMessages([]);
  }

  dispose(): void {
    this.messages = [];
    this.recentIds.clear();
  }
}
