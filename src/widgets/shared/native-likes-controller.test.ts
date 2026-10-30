import { describe, expect, test } from 'bun:test';
import { NativeLikesController } from './native-likes-controller.ts';
import type { LikeBoardEntry } from './likes-controller.ts';

describe('native likes snapshot', () => {
  test('reports degraded snapshots and clears on disconnect', () => {
    const statuses: boolean[] = [];
    const controller = new NativeLikesController({ onBoard: () => {} }, 10, (degraded) => statuses.push(degraded));
    controller.handleEnvelope({ topic: 'live.likes-ranking-full', data: { creatorId: 'c', roomId: 'r', leaders: [], integrityStatus: 'degraded' } });
    controller.handleEnvelope({ topic: 'live.disconnected' });
    expect(statuses).toEqual([true, false]);
  });
  test('uses authoritative ordering and replaces state without accumulating', () => {
    let board: LikeBoardEntry[] = [];
    const controller = new NativeLikesController({ onBoard: (entries) => { board = entries; } }, 10);
    const snapshot = { topic: 'live.likes-ranking-full', data: { creatorId: 'creator', roomId: 'room', leaders: [
      { userId: '1', uniqueId: 'ana', nickname: 'Ana', rank: 1, score: 15 },
      { userId: '2', uniqueId: 'bob', nickname: 'Bob', rank: 2, score: 10, avatarUrl: 'https://example.test/a.png' },
    ] } };
    expect(controller.handleEnvelope(snapshot)).toBe('updated');
    expect(board.map((entry) => entry.total)).toEqual([15, 10]);
    expect(board.map((entry) => entry.userId)).toEqual(['1', '2']);
    expect(controller.handleEnvelope(snapshot)).toBe('updated');
    expect(board.map((entry) => entry.total)).toEqual([15, 10]);
    expect(controller.handleEnvelope({ topic: 'live.likes-ranking-full', data: { creatorId: 'c', roomId: 'new', leaders: [] } })).toBe('updated');
    expect(board).toEqual([]);
  });
  test('renders wider native boards without accumulating feed rows', () => {
    let board: LikeBoardEntry[] = [];
    const controller = new NativeLikesController({ onBoard: (entries) => { board = entries; } }, 20);
    const leaders = Array.from({ length: 25 }, (_, index) => ({ userId: String(index + 1), uniqueId: `u${index + 1}`, nickname: `User ${index + 1}`, rank: index + 1, score: 25 - index }));
    expect(controller.handleEnvelope({ topic: 'live.likes-ranking-full', data: { creatorId: 'c', roomId: 'r', leaders } })).toBe('updated');
    expect(board).toHaveLength(20);
    expect(board[19]?.total).toBe(6);
    controller.handleEnvelope({ topic: 'live.disconnected', data: null });
    expect(board).toEqual([]);
  });
  test('rejects malformed snapshots without replacing the board', () => {
    let calls = 0;
    const controller = new NativeLikesController({ onBoard: () => { calls += 1; } }, 10);
    expect(controller.handleEnvelope({ topic: 'live.likes-ranking-full', data: { creatorId: 'c', roomId: 'r', leaders: [{ userId: '1', uniqueId: 'a', nickname: 'A', rank: 1, score: Number.MAX_SAFE_INTEGER + 1 }] } })).toBe('ignored');
    expect(calls).toBe(0);
  });
  test('omits unsafe or credential-bearing avatar URLs', () => {
    let board: LikeBoardEntry[] = [];
    const controller = new NativeLikesController({ onBoard: (entries) => { board = entries; } }, 10);
    const leaders = [
      { userId: '1', uniqueId: 'a', nickname: 'A', rank: 1, score: 2, avatarUrl: 'https://user:secret@example.com/image' },
      { userId: '2', uniqueId: 'b', nickname: 'B', rank: 2, score: 1, avatarUrl: 'javascript:alert(1)' },
    ];
    expect(controller.handleEnvelope({ topic: 'live.likes-ranking-full', data: { creatorId: 'c', roomId: 'r', leaders } })).toBe('updated');
    expect(board.map((entry) => entry.avatarUrl)).toEqual([null, null]);
  });
});
