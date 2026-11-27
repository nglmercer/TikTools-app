/** Ids shared by test fixtures (host, room, user, template, frame). */
export const FIXTURE_ID = {
  hostA: 'host-a',
  instanceA: 'instance-a',
  room1: 'room-1',
  roomA: 'room-a',
  user1: 'user-1',
  tpl1: 'tpl-1',
  frame1: 'frame-1',
  connection1: 'connection-1',
  prof1: 'prof-1',
  testHost: 'test-host',
  actFrozen: 'act-frozen',
  aOne: 'a.one',
} as const;
export type FixtureId = (typeof FIXTURE_ID)[keyof typeof FIXTURE_ID];
