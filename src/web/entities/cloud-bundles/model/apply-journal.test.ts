import { describe, expect, it } from 'vitest';
import {
  createJournal,
  isFinished,
  nextPending,
  parseJournal,
  recoverInterrupted,
  serializeJournal,
  setEntry,
  summarize,
} from './apply-journal.ts';

const id = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';
const ready = [
  { kind: 'plugin', id: 'acme.gift', version: '1.2.0', revision: null, status: 'ready' },
  { kind: 'template', id, version: null, revision: 3, status: 'ready' },
];

describe('apply journal', () => {
  it('refuses to start while any item is not ready', () => {
    expect(() => createJournal('pkg', [...ready, { ...ready[1]!, status: 'revisionChanged' }], 1)).toThrow(/revisionChanged/);
    expect(() => createJournal('pkg', [], 1)).toThrow();
  });

  it('starts every entry pending and never mutates the previous journal', () => {
    const first = createJournal('pkg', ready, 10);
    const second = setEntry(first, 0, 'done');
    expect(first.entries[0]?.status).toBe('pending');
    expect(second.entries[0]?.status).toBe('done');
    expect(nextPending(second)).toBe(1);
  });

  it('marks a step that was running at restart as interrupted, not pending', () => {
    const running = setEntry(createJournal('pkg', ready, 10), 0, 'running');
    const recovered = recoverInterrupted(running);
    expect(recovered.entries[0]?.status).toBe('interrupted');
    expect(nextPending(recovered)).toBe(1);
    expect(isFinished(recovered)).toBe(false);
  });

  it('summarizes outcomes and reports when every step has an outcome', () => {
    let journal = createJournal('pkg', ready, 10);
    journal = setEntry(journal, 0, 'failed', 'could not verify');
    journal = setEntry(journal, 1, 'skipped', 'not available');
    expect(summarize(journal)).toMatchObject({ failed: 1, skipped: 1, pending: 0 });
    expect(isFinished(journal)).toBe(true);
  });

  it('round-trips a persisted journal and refuses malformed text', () => {
    const journal = setEntry(createJournal('pkg', ready, 10), 1, 'done', 'imported');
    expect(parseJournal(serializeJournal(journal))).toEqual(journal);
    expect(parseJournal('not json')).toBeNull();
    expect(parseJournal(JSON.stringify({ packageId: 'x', startedAt: 1, entries: [{ kind: 'script', id: 'a', status: 'done' }] }))).toBeNull();
    expect(parseJournal(JSON.stringify({ packageId: 'x', startedAt: 1, entries: [{ kind: 'plugin', id: 'a', status: 'exploded' }] }))).toBeNull();
  });
});
