import { expect, it } from 'vitest';
import { contextInstance, contextName } from './context-instance.ts';
it('separates copied full-demo URLs in new browsing contexts but keeps reset reloads stable', () => {
  const first = contextInstance('example', '', () => 'first-context');
  const copied = contextInstance(first, '', () => 'second-context');
  expect(copied).not.toBe(first);
  expect(contextInstance(first, contextName(first), () => { throw new Error('Reload must not create a new namespace.'); })).toBe(first);
});
it('bounds context IDs even when the supplied instance name is already at its limit', () => {
  expect(contextInstance('x'.repeat(64), '', () => '00000000-0000-4000-8000-000000000001').length).toBeLessThanOrEqual(64);
});
