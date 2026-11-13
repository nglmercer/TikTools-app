import { expect, test } from 'bun:test';
import { indexableDocument } from './search-policy.ts';
test('search explicitly excludes historical, draft, internal and compatibility material', () => {
  const reviewed = { contentStatus: 'reviewed', audience: 'user', featureStatus: 'available' };
  expect(indexableDocument(reviewed)).toBe(true);
  for (const override of [{ featureStatus: 'historical' }, { contentStatus: 'draft' }, { audience: 'internal' }, { search: false }]) expect(indexableDocument({ ...reviewed, ...override })).toBe(false);
  expect(indexableDocument({})).toBe(false);
});
