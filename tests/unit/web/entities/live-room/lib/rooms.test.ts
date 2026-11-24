import { describe, expect, it } from 'vitest';
import type { LiveChannel } from '@/types.ts';
import { chooseRandomRoom, dedupeRooms } from '../../../../../../src/web/entities/live-room/lib/rooms.ts';

function room(uniqueId: string, roomId: string): LiveChannel {
  return { uniqueId, roomId, nickname: uniqueId, title: '', viewers: 0 };
}

describe('live rooms', () => {
  it('dedupes by room id, falling back to the handle', () => {
    expect(dedupeRooms([room('a', '1'), room('b', ''), room('c', '1'), room('b', '')]).map((entry) => entry.uniqueId)).toEqual([
      'a',
      'b',
    ]);
  });

  it('avoids the current creator unless it is the only room', () => {
    const rooms = [room('Luna', '1'), room('mango', '2')];
    expect(chooseRandomRoom(rooms, '@luna', () => 0)?.uniqueId).toBe('mango');
    expect(chooseRandomRoom([room('luna', '1')], 'luna', () => 0)?.uniqueId).toBe('luna');
    expect(chooseRandomRoom([], 'luna')).toBeNull();
  });
});
