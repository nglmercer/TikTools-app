import { describe, expect, it } from 'vitest';
import { deriveStatus, diffFields, displayLeaf, flattenLeaves } from '../../../../../../src/web/features/review-shared/model/change-review.ts';
import { defined } from '../../../../../shared/index.ts';

describe('flattenLeaves', () => {
  it('flattens nested plain objects into dot paths and keeps arrays as leaves', () => {
    expect(flattenLeaves({ a: 1, b: { c: 2, d: { e: 3 } }, f: [1, 2] })).toEqual({ a: 1, 'b.c': 2, 'b.d.e': 3, f: [1, 2] });
  });
});

describe('diffFields', () => {
  it('marks added, deleted, modified and unchanged leaves with changed first', () => {
    const fields = diffFields({ keep: 1, drop: 2, change: 'a' }, { keep: 1, change: 'b', add: true });
    expect(fields.find((field) => field.key === 'keep')?.status).toBe('unchanged');
    expect(fields.find((field) => field.key === 'drop')?.status).toBe('deleted');
    expect(fields.find((field) => field.key === 'change')?.status).toBe('modified');
    expect(fields.find((field) => field.key === 'add')?.status).toBe('added');
    expect(defined(fields[0]).status).not.toBe('unchanged');
  });

  it('compares array leaves by content', () => {
    expect(defined(diffFields({ ids: [1, 2] }, { ids: [1, 2] })[0]).status).toBe('unchanged');
    expect(defined(diffFields({ ids: [1, 2] }, { ids: [1, 3] })[0]).status).toBe('modified');
  });
});

describe('deriveStatus', () => {
  it('derives one-sided and deep-equality states', () => {
    expect(deriveStatus(undefined, { a: 1 })).toBe('added');
    expect(deriveStatus({ a: 1 }, undefined)).toBe('deleted');
    expect(deriveStatus({ a: 1 }, { a: 1 })).toBe('unchanged');
    expect(deriveStatus({ a: 1 }, { a: 2 })).toBe('modified');
    expect(deriveStatus(null, null)).toBe('unchanged');
  });
});

describe('displayLeaf', () => {
  it('renders primitives and compacts objects', () => {
    expect(displayLeaf('x')).toBe('x');
    expect(displayLeaf(3)).toBe('3');
    expect(displayLeaf(true)).toBe('true');
    expect(displayLeaf(undefined)).toBe('');
    expect(displayLeaf([1])).toBe('[1]');
  });
});
