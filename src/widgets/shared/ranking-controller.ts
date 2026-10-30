/** Presentation adapter for the host's ranking.updated snapshots.
 * It never calculates or changes scores. Call clear() on reconnect or gap;
 * the gateway periodically republishes the latest host snapshot.
 */
import { isRecord } from './event-parser.ts';

export interface RankingEntry { key: string; rank: number; score: number; label: string; userId?: string; uniqueId?: string; nickname?: string; avatarUrl?: string; giftId?: string; iconUrl?: string }
export interface RankingSnapshot {
  rankingId: string;
  schemaVersion: 1;
  creatorId: string;
  roomId: string;
  revision: number;
  instanceId?: string;
  integrityStatus: 'unknown' | 'degraded';
  paused: boolean;
  window?: { type: 'session' } | { type: 'rolling'; durationMs: number; maxEvents: number };
  rejected: number;
  dropped: number;
  limited: number;
  cooldownRejected: number;
  entries: RankingEntry[];
  totalScore: number;
  totalParticipants: number;
}

function integer(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0;
}

function displayText(value: unknown, max = 512): string | undefined {
  return typeof value === 'string' && value.length > 0 && value.length <= max ? value : undefined;
}
export function safeImageUrl(value: unknown): string | undefined {
  const text = displayText(value, 2048);
  if (!text) return undefined;
  try {
    const url = new URL(text);
    return (url.protocol === 'https:' || url.protocol === 'http:') && !url.username && !url.password ? text : undefined;
  } catch { return undefined; }
}

/** Reject unsafe integer values rather than render rounded Rust u64 scores. */
export function parseRankingSnapshot(envelope: unknown): RankingSnapshot | null {
  if (!isRecord(envelope) || envelope['topic'] !== 'ranking.updated' || !isRecord(envelope['data'])) return null;
  const snapshot = envelope['data']['snapshot'];
  if (!isRecord(snapshot) || snapshot['schemaVersion'] !== 1 || typeof snapshot['rankingId'] !== 'string'
    || !snapshot['rankingId'] || typeof snapshot['creatorId'] !== 'string' || typeof snapshot['roomId'] !== 'string'
    || !integer(snapshot['revision']) || !integer(snapshot['totalScore']) || !integer(snapshot['totalParticipants'])
    || typeof snapshot['paused'] !== 'boolean'
    || (snapshot['integrityStatus'] !== 'unknown' && snapshot['integrityStatus'] !== 'degraded')
    || !Array.isArray(snapshot['entries']) || snapshot['entries'].length > 100_000) return null;
  const diagnostics = ['rejected', 'dropped', 'limited', 'cooldownRejected'] as const;
  if (snapshot['instanceId'] !== undefined && (typeof snapshot['instanceId'] !== 'string' || !/^[0-9a-f]{32}$/i.test(snapshot['instanceId']))) return null;
  if (diagnostics.some((key) => snapshot[key] !== undefined && !integer(snapshot[key]))) return null;
  let window: RankingSnapshot['window'];
  if (snapshot['window'] !== undefined) {
    if (!isRecord(snapshot['window'])) return null;
    if (snapshot['window']['type'] === 'session') window = { type: 'session' };
    else if (snapshot['window']['type'] === 'rolling' && integer(snapshot['window']['durationMs']) && snapshot['window']['durationMs'] > 0 && integer(snapshot['window']['maxEvents']) && snapshot['window']['maxEvents'] > 0) window = { type: 'rolling', durationMs: snapshot['window']['durationMs'], maxEvents: snapshot['window']['maxEvents'] };
    else return null;
  }
  const entries: RankingEntry[] = [];
  const keys = new Set<string>();
  let displayedTotal = 0;
  for (const [index, entry] of snapshot['entries'].entries()) {
    if (!isRecord(entry) || typeof entry['key'] !== 'string' || !entry['key'] || keys.has(entry['key'])
      || !integer(entry['rank']) || entry['rank'] !== index + 1 || !integer(entry['score'])) return null;
    displayedTotal += entry['score'];
    if (!Number.isSafeInteger(displayedTotal) || displayedTotal > snapshot['totalScore']) return null;
    const previous = entries[index - 1];
    if (previous && previous.score < entry['score']) return null;
    keys.add(entry['key']);
    entries.push({ key: entry['key'], rank: entry['rank'], score: entry['score'],
      label: displayText(entry['label']) ?? entry['key'], userId: displayText(entry['userId']),
      uniqueId: displayText(entry['uniqueId']), nickname: displayText(entry['nickname']),
      avatarUrl: safeImageUrl(entry['avatarUrl']), giftId: displayText(entry['giftId']), iconUrl: safeImageUrl(entry['iconUrl']) });
  }
  if (entries.length > snapshot['totalParticipants']) return null;
  return {
    rankingId: snapshot['rankingId'], schemaVersion: 1, creatorId: snapshot['creatorId'], roomId: snapshot['roomId'],
    revision: snapshot['revision'], instanceId: snapshot['instanceId'] as string | undefined, integrityStatus: snapshot['integrityStatus'], paused: snapshot['paused'],
    window, rejected: (snapshot['rejected'] as number | undefined) ?? 0, dropped: (snapshot['dropped'] as number | undefined) ?? 0,
    limited: (snapshot['limited'] as number | undefined) ?? 0, cooldownRejected: (snapshot['cooldownRejected'] as number | undefined) ?? 0,
    entries, totalScore: snapshot['totalScore'], totalParticipants: snapshot['totalParticipants'],
  };
}

export class RankingController {
  private current: RankingSnapshot | null = null;
  private retired = new Set<string>();
  constructor(private readonly rankingId: string, private readonly onSnapshot: (snapshot: RankingSnapshot | null) => void) {}
  handleEnvelope(value: unknown): boolean {
    const snapshot = parseRankingSnapshot(value);
    if (!snapshot || snapshot.rankingId !== this.rankingId) return false;
    if (snapshot.instanceId && this.retired.has(snapshot.instanceId)) return false;
    if (this.current && snapshot.creatorId === this.current.creatorId && snapshot.roomId === this.current.roomId
      && snapshot.instanceId === this.current.instanceId && snapshot.revision <= this.current.revision) return false;
    if (this.current?.instanceId && snapshot.instanceId && this.current.instanceId !== snapshot.instanceId) {
      this.retired.add(this.current.instanceId);
      if (this.retired.size > 128) this.retired.delete(this.retired.values().next().value!);
    }
    this.current = snapshot;
    this.onSnapshot(snapshot);
    return true;
  }
  clear(): void { this.current = null; this.retired.clear(); this.onSnapshot(null); }
  dispose(): void { this.current = null; this.retired.clear(); }
}
