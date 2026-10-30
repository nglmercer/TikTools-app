import { describe, expect, test } from 'bun:test';
import { RankingController, parseRankingSnapshot } from './ranking-controller.ts';

const snapshot = (revision: number, score = 5) => ({ topic: 'ranking.updated', data: { snapshot: {
  rankingId: 'r', schemaVersion: 1, creatorId: 'c', roomId: 'room', revision, integrityStatus: 'unknown', paused: false,
  entries: [{ key: 'u', rank: 1, score }], totalScore: score, totalParticipants: 1,
} } });
describe('ranking snapshot adapter', () => {
  test('accepts only the selected ranking and ignores stale revisions until reconnect', () => {
    const received: number[] = [];
    const controller = new RankingController('r', (value) => { if (value) received.push(value.revision); });
    expect(controller.handleEnvelope(snapshot(2))).toBe(true);
    expect(controller.handleEnvelope(snapshot(2))).toBe(false);
    expect(controller.handleEnvelope(snapshot(1))).toBe(false);
    controller.clear();
    expect(controller.handleEnvelope(snapshot(1))).toBe(true);
    expect(received).toEqual([2, 1]);
  });
  test('accepts a recreated ranking and rejects updates from the retired instance', () => {
    const controller = new RankingController('r', () => {});
    const first = snapshot(20);
    const second = snapshot(1);
    Object.assign(first.data.snapshot, { instanceId: 'a'.repeat(32) });
    Object.assign(second.data.snapshot, { instanceId: 'b'.repeat(32) });
    expect(controller.handleEnvelope(first)).toBe(true);
    expect(controller.handleEnvelope(second)).toBe(true);
    expect(controller.handleEnvelope(first)).toBe(false);
    Object.assign(second.data.snapshot, { instanceId: 'invalid' });
    expect(parseRankingSnapshot(second)).toBeNull();
  });
  test('uses native labels and accepts only safe artwork URLs', () => {
    const event = snapshot(1);
    Object.assign(event.data.snapshot.entries[0]!, { label: 'Alice', nickname: 'Alice', avatarUrl: 'https://cdn.example/avatar.png' });
    expect(parseRankingSnapshot(event)?.entries[0]).toMatchObject({ key: 'u', label: 'Alice', avatarUrl: 'https://cdn.example/avatar.png' });
    Object.assign(event.data.snapshot.entries[0]!, { avatarUrl: 'javascript:alert(1)', iconUrl: 'https://user:secret@example.com/image' });
    const entry = parseRankingSnapshot(event)?.entries[0];
    expect(entry?.avatarUrl).toBeUndefined();
    expect(entry?.iconUrl).toBeUndefined();
    expect(entry?.label).toBe('Alice');
  });
  test('rejects malformed and imprecise scores', () => {
    expect(parseRankingSnapshot(snapshot(1, Number.MAX_SAFE_INTEGER + 1))).toBeNull();
    expect(parseRankingSnapshot({ topic: 'ranking.updated', data: {} })).toBeNull();
  });
  test('preserves window and accounting diagnostics', () => {
    const event = snapshot(1);
    Object.assign(event.data.snapshot, { window: { type: 'rolling', durationMs: 1000, maxEvents: 20 }, rejected: 2, dropped: 1, limited: 3, cooldownRejected: 4 });
    expect(parseRankingSnapshot(event)).toMatchObject({ window: { type: 'rolling', durationMs: 1000, maxEvents: 20 }, rejected: 2, dropped: 1, limited: 3, cooldownRejected: 4 });
    Object.assign(event.data.snapshot, { window: { type: 'rolling', durationMs: 0, maxEvents: 20 } });
    expect(parseRankingSnapshot(event)).toBeNull();
  });
});
