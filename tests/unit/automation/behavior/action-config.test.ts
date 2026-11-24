import { describe, expect, test } from 'vitest';

import {
  defaultActionConfig,
  normalizeActionConfig,
  schemaForActionType,
  uiHintsForActionType,
} from '../../../../src/automation/behavior/action-config.ts';
import type { ActionTypeDefinition } from '../../../../src/automation/behavior/types.ts';

function rangeAction(): ActionTypeDefinition {
  return {
    id: 'audio.play.process',
    title: { default: 'Play a sound', i18key: 'x' },
    description: { default: 'Play', i18key: 'x' },
    tag: 'audio',
    source: { kind: 'builtin' },
    requiredCapabilities: [],
    fields: [
      {
        key: 'volume',
        label: { default: 'Volume', i18key: 'x' },
        kind: 'range',
        value: '1',
        min: 0,
        max: 1,
        step: 0.05,
      },
    ],
  };
}

describe('range action fields', () => {
  test('schema carries the number type with bounds', () => {
    const schema = schemaForActionType(rangeAction()).properties as Record<string, Record<string, unknown>>;
    expect(schema['volume']).toMatchObject({ type: 'number', minimum: 0, maximum: 1, multipleOf: 0.05 });
  });

  test('ui hints keep the range kind for the renderer', () => {
    const hints = uiHintsForActionType(rangeAction()).fields as Record<string, Record<string, unknown>>;
    expect(hints['volume']?.['kind']).toBe('range');
  });

  test('defaults and normalization stay numeric', () => {
    expect(defaultActionConfig(rangeAction())).toMatchObject({ volume: 1 });
    expect(normalizeActionConfig(rangeAction(), { volume: 0.5 })).toMatchObject({ volume: 0.5 });
    expect(normalizeActionConfig(rangeAction(), { volume: 'garbage' })).toMatchObject({ volume: 1 });
    expect(normalizeActionConfig(rangeAction(), {})).toMatchObject({ volume: 1 });
  });
});
