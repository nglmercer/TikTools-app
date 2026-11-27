/** Ids shared by test fixtures (host, room, user, template, frame). */
export const FIXTURE_ID = {
  hostA: 'host-a',
  instanceA: 'instance-a',
  room1: 'room-1',
  roomA: 'room-a',
  user1: 'user-1',
  tpl1: 'tpl-1',
  frame1: 'frame-1',
} as const;
export type FixtureId = (typeof FIXTURE_ID)[keyof typeof FIXTURE_ID];
