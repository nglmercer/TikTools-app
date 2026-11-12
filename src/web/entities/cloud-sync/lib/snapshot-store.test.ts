import { beforeEach, describe, expect, it } from 'vitest';
import { advanceSnapshot, readSnapshot, snapshotRevision, writeSnapshot } from './snapshot-store.ts';

describe('snapshot store', () => {
  beforeEach(() => localStorage.clear());

  it('reads back what was written and treats missing or broken data as no snapshot', () => {
    expect(readSnapshot('s')).toBeNull();
    writeSnapshot('s', { a: { n: 1 } });
    expect(readSnapshot('s')).toEqual({ a: { n: 1 } });
    localStorage.setItem('tiktools.sync.snapshot.s', '[1]');
    expect(readSnapshot('s')).toBeNull();
    localStorage.setItem('tiktools.sync.snapshot.s', '{nope');
    expect(readSnapshot('s')).toBeNull();
  });

  it('does not bump the revision when a snapshot is only seeded', () => {
    const before = snapshotRevision();
    writeSnapshot('s', { a: 1 });
    expect(snapshotRevision()).toBe(before);
  });

  it('replaces the reference and bumps the revision when it is advanced', () => {
    writeSnapshot('s', { a: { n: 1 } });
    const before = snapshotRevision();
    advanceSnapshot('s', { a: { n: 2 }, b: { n: 3 } });
    expect(readSnapshot('s')).toEqual({ a: { n: 2 }, b: { n: 3 } });
    expect(snapshotRevision()).toBe(before + 1);
  });
});
