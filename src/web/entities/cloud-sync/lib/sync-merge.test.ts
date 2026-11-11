import { describe, expect, it } from 'vitest';
import { defineSyncAdapter, noSyncCapabilities } from '../api/sync-adapter.ts';
import { buildFieldChoices, groupChanges, mergeChoices, ROOT_FIELD } from './sync-merge.ts';

const base = { id: 'a', name: 'Base', enabled: true, volume: 5 };
const local = { id: 'a', name: 'Mine', enabled: true, volume: 7 };
const cloud = { id: 'a', name: 'Theirs', enabled: false, volume: 5 };

describe('buildFieldChoices', () => {
  it('lists only the fields where Local and Cloud disagree, with the side that changed as default', () => {
    const fields = buildFieldChoices(base, local, cloud);
    expect(fields.map((field) => [field.key, field.choice])).toEqual([
      ['name', 'local'], // both changed it: Local wins by default
      ['enabled', 'cloud'], // only Cloud changed it
      ['volume', 'local'], // only Local changed it
    ]);
    expect(fields[0]).toMatchObject({ base: 'Base', local: 'Mine', cloud: 'Theirs' });
  });

  it('skips fields both sides changed to the same value', () => {
    expect(buildFieldChoices(base, { ...base, volume: 9 }, { ...base, volume: 9 })).toEqual([]);
  });

  it('treats a value that is not an object as one field', () => {
    expect(buildFieldChoices(1, 2, 3)).toEqual([{ key: ROOT_FIELD, base: 1, local: 2, cloud: 3, choice: 'local' }]);
  });

  it('handles added and removed keys', () => {
    const fields = buildFieldChoices({ a: 1 }, { a: 1, mine: true }, { a: 1, theirs: true });
    expect(fields.map((field) => field.key).sort()).toEqual(['mine', 'theirs']);
  });
});

describe('mergeChoices', () => {
  it('builds the merged object from the picked side of every field', () => {
    const fields = buildFieldChoices(base, local, cloud);
    const merged = mergeChoices(local, cloud, fields, { name: 'cloud', enabled: 'base', volume: 'local' });
    expect(merged).toEqual({ id: 'a', name: 'Theirs', enabled: true, volume: 7 });
  });

  it('with every default pick keeps local edits and takes Cloud-only edits', () => {
    const fields = buildFieldChoices(base, local, cloud);
    const picks = Object.fromEntries(fields.map((field) => [field.key, field.choice]));
    expect(mergeChoices(local, cloud, fields, picks)).toEqual({ id: 'a', name: 'Mine', enabled: false, volume: 7 });
  });

  it('drops a key whose picked side no longer has it', () => {
    const fields = buildFieldChoices({ a: 1, b: 2 }, { a: 1, b: 3 }, { a: 1 });
    expect(mergeChoices({ a: 1, b: 3 }, { a: 1 }, fields, { b: 'cloud' })).toEqual({ a: 1 });
  });

  it('returns the whole picked value for a non-object', () => {
    const fields = buildFieldChoices(1, 2, 3);
    expect(mergeChoices(2, 3, fields, { [ROOT_FIELD]: 'cloud' })).toBe(3);
  });
});

describe('groupChanges', () => {
  it('orders groups To send, To fetch, Conflicts and skips empty ones', () => {
    const change = (id: string, side: 'local' | 'remote' | 'conflict') => ({ id, kind: 'x', name: id, status: 'modified' as const, side });
    const groups = groupChanges([change('c', 'conflict'), change('l', 'local'), change('l2', 'local')]);
    expect(groups.map((group) => [group.side, group.changes.length])).toEqual([['local', 2], ['conflict', 1]]);
  });
});

describe('defineSyncAdapter', () => {
  it('declares every capability false unless stated', () => {
    const adapter = defineSyncAdapter({ scope: { id: 's', label: 'S' }, kindLabel: (kind) => kind, list: async () => [], capabilities: { sync: true } });
    expect(adapter.capabilities).toEqual({ ...noSyncCapabilities, sync: true });
  });
});
