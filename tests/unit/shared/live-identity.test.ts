import { expect, test } from 'vitest';
import { liveMessageIdentity } from '../../../src/shared/live-identity.ts';
import { EVENT_KIND } from '../../shared/index.ts';
test('identities preserve large IDs, event type and room/creator scope', () => {
  const id = '18446744073709551615';
  const key = liveMessageIdentity(EVENT_KIND.tiktokGift, id, '1', 'a');
  expect(key).not.toBe(liveMessageIdentity(EVENT_KIND.tiktokLike, id, '1', 'a'));
  expect(key).not.toBe(liveMessageIdentity(EVENT_KIND.tiktokGift, id, '2', 'a'));
  expect(key).not.toBe(liveMessageIdentity(EVENT_KIND.tiktokGift, id, '1', 'b'));
  for (const missing of [undefined, null, '', '0', '000', 42]) expect(liveMessageIdentity('gift', missing)).toBeNull();
});
