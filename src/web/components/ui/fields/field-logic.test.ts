import { describe, expect, test } from 'bun:test';

import {
  describeField,
  fieldControlId,
  fieldMessageIds,
  parseNumberDraft,
  resolveFieldSize,
  roundToStep,
} from './field-logic.ts';

describe('field size + ids', () => {
  test('resolveFieldSize defaults to md and rejects unknown sizes', () => {
    expect(resolveFieldSize(undefined)).toBe('md');
    expect(resolveFieldSize('sm')).toBe('sm');
    expect(resolveFieldSize('lg')).toBe('lg');
    expect(resolveFieldSize('xl')).toBe('md');
    expect(resolveFieldSize(null)).toBe('md');
  });

  test('fieldControlId prefers id, then name, then fallback', () => {
    expect(fieldControlId({ id: 'a', name: 'b' }, 'tt-fallback')).toBe('a');
    expect(fieldControlId({ name: 'voice' }, 'tt-fallback')).toBe('tt-voice');
    expect(fieldControlId({}, 'tt-fallback')).toBe('tt-fallback');
    expect(fieldControlId({ id: '', name: 'voice' }, 'tt-fallback')).toBe('tt-voice');
  });

  test('fieldMessageIds derive stable description/error ids', () => {
    expect(fieldMessageIds('tt-voice')).toEqual({
      descriptionId: 'tt-voice-description',
      errorId: 'tt-voice-error',
    });
  });

  test('describeField joins rendered ids and stays undefined when empty', () => {
    expect(describeField(['a', undefined, 'b', false])).toBe('a b');
    expect(describeField([undefined, false])).toBeUndefined();
    expect(describeField(['a', '', 'b'])).toBe('a b');
    expect(describeField([])).toBeUndefined();
  });
});

describe('number drafts', () => {
  test('parseNumberDraft keeps intermediate typing states editable', () => {
    expect(parseNumberDraft('')).toEqual({ kind: 'empty' });
    expect(parseNumberDraft('  ')).toEqual({ kind: 'empty' });
    expect(parseNumberDraft('-')).toEqual({ kind: 'empty' });
    expect(parseNumberDraft('.')).toEqual({ kind: 'empty' });
    expect(parseNumberDraft('-.')).toEqual({ kind: 'empty' });
    expect(parseNumberDraft('1.')).toEqual({ kind: 'number', value: 1 });
    expect(parseNumberDraft('abc')).toEqual({ kind: 'invalid' });
    expect(parseNumberDraft('12abc')).toEqual({ kind: 'invalid' });
    expect(parseNumberDraft('12.5')).toEqual({ kind: 'number', value: 12.5 });
    expect(parseNumberDraft('1e3')).toEqual({ kind: 'number', value: 1000 });
  });

  test('roundToStep follows step precision', () => {
    expect(roundToStep(1.234)).toBe(1);
    expect(roundToStep(1.234, 1)).toBe(1);
    expect(roundToStep(1.234, 0.1)).toBe(1.2);
    expect(roundToStep(1.235, 0.05)).toBe(1.24);
    expect(roundToStep(1.234, 0)).toBe(1);
    expect(roundToStep(-1.26, 0.1)).toBe(-1.3);
  });
});
