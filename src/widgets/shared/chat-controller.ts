/**
 * Chat overlay controller: validates routed chat events, drops historical,
 * duplicate, and empty deliveries, and keeps a bounded message list for
 * the overlay. Rendering-agnostic; the Vue component only observes the
 * snapshot the handler receives.
 */

import { DEFAULT_CHAT_MESSAGE_LIMIT, DEFAULT_RECENT_IDS_CAPACITY } from './config.ts';
import { RecentEventIds } from './alert-queue.ts';
import { routeLiveEvent } from './event-router.ts';
import { displayNameFor, eventIdentity, handleFor } from './event-types.ts';
import type { LiveEmote } from '../../shared/emotes/types.ts';

export interface ChatMessageView {
  id: string;
  displayName: string;
  uniqueId: string;
  avatarUrl: string | null;
  text: string;
  emotes?: LiveEmote[];
  emoteOnly?: boolean;
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
    if (!this.recentIds.add(eventIdentity(event))) return 'duplicate';
    const emoteOnly = event.type === 'tiktok.emote_chat';
    const text = event.type !== 'tiktok.emote_chat' ? event.data.comment : '';
    const emotes = (event.data.emotes ?? []).map((emote) => ({
      id: emote.id,
      ...(typeof emote.shortcode === 'string' ? { shortcode: emote.shortcode } : {}),
      ...(typeof emote.imageUrl === 'string' ? { imageUrl: emote.imageUrl } : {}),
      ...(typeof emote.index === 'number' ? { index: emote.index } : {}),
      ...(typeof emote.emoteType === 'number' ? { emoteType: emote.emoteType } : {}),
      ...(typeof emote.packageId === 'string' ? { packageId: emote.packageId } : {}),
    }));
    const hasRenderableEmote = emotes.length > 0;
    if (!text.trim() && !hasRenderableEmote) return 'ignored';
    this.messages = [
      ...this.messages,
      {
        id: event.id,
        displayName: displayNameFor(event.user),
        uniqueId: handleFor(event.user),
        avatarUrl: event.user?.avatarUrl ?? null,
        text,
        ...(emotes.length > 0 ? { emotes } : {}),
        ...(emoteOnly ? { emoteOnly: true } : {}),
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
