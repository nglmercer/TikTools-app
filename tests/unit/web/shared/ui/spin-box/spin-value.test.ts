import { describe, expect, it } from 'vitest';
import { formatSpinValue, normalizeSpinValue, parseSpinValue, precisionOf, stepSpinValue } from '../../../../../../src/web/shared/ui/spin-box/spin-value.ts';

describe('spin values', () => {
  it('derives precision from the step', () => {
    expect(precisionOf(1)).toBe(0);
    expect(precisionOf(0.25)).toBe(2);
  });

  it('clamps and rounds to the range precision', () => {
    const range = { min: 0, max: 10, step: 0.5, precision: 1 };
    expect(normalizeSpinValue(12, range)).toBe(10);
    expect(normalizeSpinValue(-3, range)).toBe(0);
    expect(normalizeSpinValue(3.14159, range)).toBe(3.1);
  });

  it('steps without floating point drift', () => {
    expect(stepSpinValue(0.2, 1, { step: 0.1, precision: 1 })).toBe(0.3);
  });

  it('parses user input leniently', () => {
    expect(parseSpinValue(' 300 ')).toBe(300);
    expect(parseSpinValue('1,5')).toBe(1.5);
    expect(parseSpinValue('-')).toBeNull();
    expect(parseSpinValue('abc')).toBeNull();
  });

  it('formats with fixed precision', () => {
    expect(formatSpinValue(2, 2)).toBe('2.00');
  });
});
