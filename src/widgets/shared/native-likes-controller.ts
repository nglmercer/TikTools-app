/** Renders the native, locally observed like leaderboard; never sums events. */
import { isRecord } from './event-parser.ts';
import { safeImageUrl } from './ranking-controller.ts';
import type { LikeBoardEntry, LikesControllerHandlers } from './likes-controller.ts';

export class NativeLikesController {
  constructor(private readonly handlers: LikesControllerHandlers, private readonly maxEntries: number, private readonly onIntegrity?: (degraded: boolean) => void) {}
  handleEnvelope(value: unknown): 'updated' | 'ignored' {
    if (isRecord(value) && value['topic'] === 'live.disconnected') { this.clear(); return 'updated'; }
    if (!isRecord(value) || value['topic'] !== 'live.likes-ranking-full' || !isRecord(value['data'])) return 'ignored';
    const data = value['data'];
    if (typeof data['creatorId'] !== 'string' || typeof data['roomId'] !== 'string' || !Array.isArray(data['leaders']) || data['leaders'].length > 500) return 'ignored';
    const entries: LikeBoardEntry[] = [];
    const keys = new Set<string>();
    for (const [index, entry] of data['leaders'].entries()) {
      if (!isRecord(entry) || typeof entry['userId'] !== 'string' || !entry['userId'] || keys.has(entry['userId'])
        || entry['rank'] !== index + 1 || typeof entry['score'] !== 'number' || !Number.isSafeInteger(entry['score']) || entry['score'] < 0
        || typeof entry['uniqueId'] !== 'string' || typeof entry['nickname'] !== 'string'
        || (entry['avatarUrl'] != null && typeof entry['avatarUrl'] !== 'string')) return 'ignored';
      keys.add(entry['userId']);
      entries.push({ userId: entry['userId'], uniqueId: entry['uniqueId'], displayName: entry['nickname'] || entry['uniqueId'], avatarUrl: safeImageUrl(entry['avatarUrl']) ?? null, total: entry['score'] });
    }
    this.handlers.onBoard(entries.slice(0, this.maxEntries));
    this.onIntegrity?.(data['integrityStatus'] === 'degraded');
    return 'updated';
  }
  clear(): void { this.handlers.onBoard([]); this.onIntegrity?.(false); }
  dispose(): void {}
}
