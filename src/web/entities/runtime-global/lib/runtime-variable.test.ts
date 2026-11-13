import { describe, expect, it } from 'vitest';
import { isRuntimeVariable, variableText, type RuntimeVariable } from './runtime-variable.ts';
import { variableFromRow } from '@/features/edit-runtime-globals/model/globals-draft.ts';

describe('lossless native numeric variables', () => {
  it('uses the native exact text after JavaScript parses a large integer', () => {
    const value = JSON.parse('{"type":"number","value":9223372036854775807,"numberText":"9223372036854775807"}') as RuntimeVariable;
    expect(variableText(value)).toBe('9223372036854775807');
    expect(isRuntimeVariable(value)).toBe(true);
    expect(variableFromRow({ id: 1, key: 'counter', kind: 'number', value: variableText(value) })).toMatchObject({ type: 'number', numberText: '9223372036854775807' });
  });
  it('keeps the previous JSON shape for ordinary numbers', () => {
    expect(variableFromRow({ id: 1, key: 'port', kind: 'number', value: '46665' })).toEqual({ type: 'number', value: 46665 });
    expect(isRuntimeVariable({ type: 'number', value: 46665 })).toBe(true);
  });
  it('refuses an older host numeric value that cannot be displayed or saved safely', () => {
    expect(isRuntimeVariable(JSON.parse('{"type":"number","value":9223372036854775807}'))).toBe(false);
  });
  it('rejects inconsistent or nonnumeric exact text', () => {
    for (const numberText of ['true', '"private"', '1e400', '12']) {
      expect(isRuntimeVariable({ type: 'number', value: 9223372036854776000, numberText })).toBe(false);
    }
  });
});
