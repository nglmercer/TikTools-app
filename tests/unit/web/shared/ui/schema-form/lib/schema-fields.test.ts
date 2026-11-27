import { describe, expect, it } from 'vitest';
import type { JsonObject } from '../../../../../../../src/automation/types.ts';
import {
  acceptsValue,
  applies,
  describeField,
  describeForm,
  numericValue,
  parseJsonField,
  textValue,
  withSchemaDefaults,
} from '../../../../../../../src/web/shared/ui/schema-form/lib/schema-fields.ts';

const localize = (value: unknown) => (typeof value === 'string' ? value : '');

function kindOf(
  schema: JsonObject,
  hint?: JsonObject,
  dynamic?: {
    value: string;
    label: string
  }[],
) {
  return describeField('field', schema, hint, dynamic, localize).kind;
}

describe('schema fields', () => {
  it('maps schema and hints to a control kind', () => {
    expect(kindOf({ type: 'boolean' })).toBe('boolean');
    expect(kindOf({ type: 'string' }, { picker: 'gift' })).toBe('gift');
    expect(kindOf({ type: 'string' }, { kind: 'media' })).toBe('media');
    expect(kindOf({ type: 'object', additionalProperties: { type: 'string' } })).toBe('keyvalue');
    expect(kindOf({ type: 'string', format: 'date' })).toBe('date');
    expect(kindOf({ type: 'string', format: 'time' })).toBe('time');
    expect(kindOf({ type: 'string', format: 'color' })).toBe('color');
    expect(kindOf({ type: 'string', secret: true })).toBe('secret');
    expect(kindOf({ type: 'number' }, { kind: 'range' })).toBe('range');
    expect(kindOf({ type: 'array', items: { enum: ['a', 'b'] } })).toBe('multiselect');
    expect(kindOf({ type: 'array', items: { type: 'string' } })).toBe('tags');
    expect(kindOf({ type: 'string', enum: ['a'] })).toBe('select');
    expect(kindOf({ type: 'string' }, undefined, [{ value: 'x', label: 'X' }])).toBe('search');
    expect(kindOf({ type: 'array' })).toBe('json');
    expect(kindOf({ type: 'string', format: 'multiline' })).toBe('textarea');
    expect(kindOf({ type: 'integer' })).toBe('number');
    expect(kindOf({ type: 'string' })).toBe('text');
  });

  it('describes labels, limits and media settings', () => {
    const field = describeField(
      'clip',
      { title: 'Clip', description: 'Sound to play', maximum: 10, minimum: 1, multipleOf: 0.5 },
      { kind: 'media', mediaKind: 'video', mode: 'directory', extensions: ['mp4', 3] },
      undefined,
      localize,
    );
    expect(field).toMatchObject({ label: 'Clip', hint: 'Sound to play', min: 1, max: 10, step: 0.5 });
    expect(field.media).toEqual({ mode: 'directory', kind: 'video', extensions: ['mp4'] });
    expect(describeField('raw', {}, undefined, undefined, localize).label).toBe('raw');
  });

  it('evaluates showIf conditions', () => {
    expect(applies(undefined, {})).toBe(true);
    expect(applies({ key: 'mode', equals: ['a', 'b'] }, { mode: 'b' })).toBe(true);
    expect(applies({ key: 'mode', equals: ['a'] }, { mode: 'b' })).toBe(false);
    expect(applies({ key: 'mode', notEquals: ['a'] }, { mode: 'a' })).toBe(false);
  });

  it('splits visible fields into basic and advanced groups', () => {
    const schema = { properties: { mode: { type: 'string' }, url: { type: 'string' }, retries: { type: 'number' } } };
    const hints = { fields: { url: { showIf: { key: 'mode', equals: ['remote'] } }, retries: { advanced: true } } };
    const local = describeForm(schema, hints, { mode: 'local' }, {}, localize);
    expect(local.basic.map((field) => field.key)).toEqual(['mode']);
    expect(local.advanced.map((field) => field.key)).toEqual(['retries']);
    expect(describeForm(schema, hints, { mode: 'remote' }, {}, localize).basic.map((field) => field.key)).toEqual(['mode', 'url']);
  });

  it('guards and converts values', () => {
    const number = describeField('n', { type: 'number', default: 5 }, undefined, undefined, localize);
    const json = describeField('j', { type: 'array' }, undefined, undefined, localize);
    expect(acceptsValue(number, '3')).toBe(false);
    expect(acceptsValue(number, 3)).toBe(true);
    expect(numericValue('', number)).toBe(5);
    expect(numericValue('7', number)).toBe(7);
    expect(textValue({ a: 1 })).toBe('{\n  "a": 1\n}');
    expect(parseJsonField('[1]', json)).toEqual([1]);
    expect(parseJsonField('[1', json)).toBe('[1');
  });

  it('fills scalar defaults without touching secrets or set values', () => {
    const schema = {
      properties: {
        host: { type: 'string', default: 'localhost' },
        port: { type: 'number', default: 8080 },
        token: { type: 'string', secret: true, default: 'x' },
        list: { type: 'array', default: [] },
      },
    };
    expect(withSchemaDefaults({ port: 1 }, schema)).toEqual({ host: 'localhost', port: 1 });
    const complete = { host: 'a', port: 2 };
    expect(withSchemaDefaults(complete, schema)).toBe(complete);
  });
});
