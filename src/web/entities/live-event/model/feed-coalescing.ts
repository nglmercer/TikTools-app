import type { Ref } from 'vue';
import type { UiEvent } from '../../../../shared/messages.ts';
import type { DisplayEvent } from '@/types.ts';

interface CoalescingSource {
  staged: () => DisplayEvent[];
  events: Ref<DisplayEvent[]>;
  session: () => { room: string; creator: string };
  stage: (event: DisplayEvent) => void;
}

/** Folds visual rows only; journal acceptance and delivery accounting remain upstream. */
export function createFeedCoalescer(source: CoalescingSource) {
  const giftGroupKey = (event: UiEvent): string | null => {
    const group = event.giftDetails?.groupId?.trim() ?? '';
    if (!group || /^0+$/.test(group)) return null;
    const giftId = event.giftDetails?.giftId?.trim() ?? '';
    if (!giftId || /^0+$/.test(giftId)) return null;
    return JSON.stringify([event.roomId ?? source.session().room, event.creatorId ?? source.session().creator, event.author, giftId, group]);
  };

  const applyGiftProgress = (target: DisplayEvent, update: UiEvent): void => {
    const count = update.giftDetails?.count ?? target.giftDetails?.count ?? 1;
    const diamonds = update.giftDetails?.diamonds ?? target.giftDetails?.diamonds ?? 0;
    target.giftDetails = {
      ...target.giftDetails,
      name: update.giftDetails?.name ?? target.giftDetails?.name ?? 'Gift',
      count,
      diamonds,
      giftId: update.giftDetails?.giftId ?? target.giftDetails?.giftId,
      imageUrl: update.giftDetails?.imageUrl ?? target.giftDetails?.imageUrl,
      groupId: update.giftDetails?.groupId ?? target.giftDetails?.groupId,
      repeatEnd: update.giftDetails?.repeatEnd ?? target.giftDetails?.repeatEnd,
      streakable: update.giftDetails?.streakable ?? target.giftDetails?.streakable,
    };
    target.text = update.text;
    if (update.nickname !== undefined) target.nickname = update.nickname;
    if (update.avatarUrl !== undefined) target.avatarUrl = update.avatarUrl;
    if (update.i18nKey !== undefined) target.i18nKey = update.i18nKey;
    if (update.i18nParams !== undefined) target.i18nParams = update.i18nParams;
    if (update.msgId !== undefined) target.msgId = update.msgId;
    target.receivedAt = Date.now();
  };

  const tryMergeGiftStreak = (event: UiEvent): boolean => {
    if (event.kind !== 'gift') return false;
    const key = giftGroupKey(event);
    if (key === null) return false;
    for (let i = source.staged().length - 1; i >= 0; i -= 1) {
      const candidate = source.staged()[i]!;
      if (candidate.kind === 'gift' && giftGroupKey(candidate) === key) {
        applyGiftProgress(candidate, event);
        return true;
      }
    }
    const committed = source.events.value;
    for (let i = committed.length - 1; i >= 0; i -= 1) {
      const candidate = committed[i]!;
      if (candidate.kind === 'gift' && giftGroupKey(candidate) === key) {
        applyGiftProgress(candidate, event);
        source.events.value = [...committed];
        return true;
      }
    }
    return false;
  };

  // A like is not a TikTok gift combo. Only coalesce its visual rows when
  // the backend supplies a reliable per-viewer total and stable viewer key.
  // The feed journal, automation rules, points and analytics retain each packet.
  const LIKE_BURST_WINDOW_MS = 3_000;
  const likeGroupKey = (event: UiEvent): string | null => {
    if (event.kind !== 'like' || !event.likeViewerKey ||
        !Number.isSafeInteger(event.likeUserTotal) || (event.likeUserTotal ?? 0) <= 0) return null;
    return JSON.stringify([
      event.creatorId ?? source.session().creator,
      event.roomId ?? source.session().room,
      event.likeViewerKey,
    ]);
  };

  const tryMergeLikeBurst = (event: UiEvent, cursor?: number): boolean => {
    const key = likeGroupKey(event);
    if (key === null) return false;
    const now = Date.now();
    const canMerge = (candidate: DisplayEvent): boolean => {
      const elapsed = now - candidate.receivedAt;
      return likeGroupKey(candidate) === key &&
        elapsed >= 0 && elapsed <= LIKE_BURST_WINDOW_MS &&
        candidate.likeUserTotal !== undefined &&
        event.likeUserTotal! > candidate.likeUserTotal;
    };
    const updatedRow = (candidate: DisplayEvent): DisplayEvent => ({
      ...candidate,
      ...event,
      id: candidate.id,
      feedCursor: cursor ?? event.feedCursor ?? candidate.feedCursor,
      receivedAt: now,
    });

    for (let i = source.staged().length - 1; i >= 0; i -= 1) {
      const candidate = source.staged()[i]!;
      if (!canMerge(candidate)) continue;
      source.staged().splice(i, 1);
      source.staged().push(updatedRow(candidate));
      return true;
    }

    const committed = source.events.value;
    for (let i = committed.length - 1; i >= 0; i -= 1) {
      const candidate = committed[i]!;
      if (!canMerge(candidate)) continue;
      // Re-stage the existing row after any already staged events to reflect
      // the latest activity without duplicating the user's row.
      source.events.value = [...committed.slice(0, i), ...committed.slice(i + 1)];
      source.stage(updatedRow(candidate));
      return true;
    }
    return false;
  };

  return { tryMergeGiftStreak, tryMergeLikeBurst };
}
