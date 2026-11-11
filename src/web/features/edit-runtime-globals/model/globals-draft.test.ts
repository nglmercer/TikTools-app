import { describe, expect, it } from 'vitest';
import { nextTick, ref } from 'vue';
import { globalsFromRows, rowsFromGlobals, useGlobalsDraft, validateGlobalRows } from './globals-draft.ts';

describe('globals draft rows', () => {
  it('builds sorted rows and converts them back ignoring empty keys', () => {
    const rows = rowsFromGlobals({ port: '8080', host: 'localhost' });
    expect(rows.map((row) => row.key)).toEqual(['host', 'port']);
    expect(globalsFromRows([...rows, { id: -1, key: '', value: '' }])).toEqual({ host: 'localhost', port: '8080' });
  });

  it('flags missing, invalid and duplicate keys', () => {
    const errors = validateGlobalRows([
      { id: 1, key: '', value: '' },
      { id: 2, key: '', value: 'orphan' },
      { id: 3, key: '9lives', value: '' },
      { id: 4, key: 'port', value: '1' },
      { id: 5, key: 'port', value: '2' },
    ]);
    expect([...errors]).toEqual([
      [2, 'emptyKey'],
      [3, 'invalidKey'],
      [4, 'duplicateKey'],
      [5, 'duplicateKey'],
    ]);
  });
});

describe('useGlobalsDraft', () => {
  it('follows the source until edited', async () => {
    const source = ref<Record<string, string>>({ a: '1' });
    const draft = useGlobalsDraft(source);
    source.value = { a: '1', b: '2' };
    await nextTick();
    expect(draft.rows.value.map((row) => row.key)).toEqual(['a', 'b']);
    draft.update(draft.rows.value[0]!.id, { value: 'changed' });
    source.value = { z: '9' };
    await nextTick();
    expect(draft.dirty.value).toBe(true);
    expect(draft.rows.value.map((row) => row.key)).toEqual(['a', 'b']);
    draft.markSaved();
    source.value = { z: '9' };
    await nextTick();
    expect(draft.rows.value.map((row) => row.key)).toEqual(['z']);
  });

  it('blocks saving while any row is invalid', () => {
    const draft = useGlobalsDraft(ref({}));
    draft.add();
    expect(draft.blocked.value).toBe(false);
    draft.update(draft.rows.value[0]!.id, { value: 'needs key' });
    expect(draft.blocked.value).toBe(true);
    draft.remove(draft.rows.value[0]!.id);
    expect(draft.blocked.value).toBe(false);
    expect(draft.dirty.value).toBe(true);
  });

  it('discards unsaved edits back to the stored values', () => {
    const draft = useGlobalsDraft(ref({ a: '1' }));
    draft.update(draft.rows.value[0]!.id, { value: 'changed' });
    draft.add();
    draft.discard();
    expect(draft.rows.value.map((row) => [row.key, row.value])).toEqual([['a', '1']]);
    expect(draft.dirty.value).toBe(false);
  });
});
