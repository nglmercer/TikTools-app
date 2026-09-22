import { describe, expect, test } from 'bun:test';

import { cloneDesign, EditorHistory } from './history.ts';

describe('editor history', () => {
  test('clones designs without sharing nested objects', () => {
    const source = { background: '#112233', avatar: { size: 64 } };
    const copy = cloneDesign(source);
    expect(copy).toEqual(source);
    copy.avatar!.size = 32;
    expect(source.avatar!.size).toBe(64);
  });

  test('undo restores pre-change snapshots and redo reapplies', () => {
    const history = new EditorHistory();
    expect(history.canUndo).toBe(false);
    expect(history.undo({})).toBeNull();

    history.commit({ background: '#111111' });
    history.commit({ background: '#222222' });
    expect(history.canUndo).toBe(true);

    expect(history.undo({ background: '#333333' })).toEqual({ background: '#222222' });
    expect(history.undo({ background: '#222222' })).toEqual({ background: '#111111' });
    expect(history.canUndo).toBe(false);
    expect(history.undo({ background: '#111111' })).toBeNull();

    expect(history.canRedo).toBe(true);
    expect(history.redo({ background: '#111111' })).toEqual({ background: '#222222' });
    expect(history.redo({ background: '#222222' })).toEqual({ background: '#333333' });
    expect(history.canRedo).toBe(false);
  });

  test('a new commit clears the redo branch', () => {
    const history = new EditorHistory();
    history.commit({ radius: 1 });
    history.undo({ radius: 2 });
    expect(history.canRedo).toBe(true);
    history.commit({ radius: 2 });
    expect(history.canRedo).toBe(false);
  });

  test('history is bounded to the capacity', () => {
    const history = new EditorHistory(3);
    for (let step = 0; step < 5; step += 1) history.commit({ radius: step });
    expect(history.depth).toBe(3);
    expect(history.undo({ radius: 99 })).toEqual({ radius: 4 });
  });

  test('snapshots can carry editor UI state alongside the draft', () => {
    const history = new EditorHistory<{ draft: { radius?: number }; visible: string[] }>();
    history.commit({ draft: {}, visible: ['message'] });
    const restored = history.undo({ draft: { radius: 8 }, visible: ['message', 'title'] });
    expect(restored).toEqual({ draft: {}, visible: ['message'] });
    expect(history.redo({ draft: {}, visible: ['message'] })).toEqual({
      draft: { radius: 8 },
      visible: ['message', 'title'],
    });
  });
});
