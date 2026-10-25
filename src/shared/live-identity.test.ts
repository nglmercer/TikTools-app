import { expect, test } from 'bun:test';
import { liveMessageIdentity } from './live-identity.ts';
test('identities preserve large IDs, event type and room/creator scope', () => {
  const id = '18446744073709551615';
  const key = liveMessageIdentity('tiktok.gift', id, '1', 'a');
  expect(key).not.toBe(liveMessageIdentity('tiktok.like', id, '1', 'a'));
  expect(key).not.toBe(liveMessageIdentity('tiktok.gift', id, '2', 'a'));
  expect(key).not.toBe(liveMessageIdentity('tiktok.gift', id, '1', 'b'));
  for (const missing of [undefined, null, '', '0', '000', 42]) expect(liveMessageIdentity('gift', missing)).toBeNull();
});
