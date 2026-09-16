import { expect, test } from 'bun:test';

import { formatJsonText, shouldFormatPastedJson, tokenizeJson, validateJsonText } from './code-editor-logic.ts';

test('formatJsonText pretty-prints valid JSON', () => {
  expect(formatJsonText('{"b":2,"a":1}')).toBe('{\n  "b": 2,\n  "a": 1\n}');
});

test('formatJsonText keeps templated strings intact', () => {
  expect(formatJsonText('{"usuario": "{{ event.user.uniqueId }}"}')).toContain('"{{ event.user.uniqueId }}"');
});

test('formatJsonText returns null for broken input', () => {
  expect(formatJsonText('{"a": }')).toBeNull();
  expect(formatJsonText('')).toBeNull();
  expect(formatJsonText('plain text')).toBeNull();
});

test('tokenizeJson marks keys vs strings', () => {
  const kinds = tokenizeJson('"usuario": "luna"').map((token) => token.cls);
  expect(kinds).toEqual(['codeed-key', 'codeed-punct', 'codeed-ws', 'codeed-str']);
});

test('tokenizeJson marks numbers, literals and punctuation', () => {
  const kinds = tokenizeJson('{"n": 1716382910, "ok": true, "x": null}').map((token) => token.cls);
  expect(kinds).toContain('codeed-num');
  expect(kinds).toContain('codeed-lit');
  expect(kinds).toContain('codeed-key');
});

test('tokenizeJson never throws on broken input', () => {
  for (const chunk of ['{', '"unterminated', '{{ event.user', '12.3.4', '   ', '}']) {
    const tokens = tokenizeJson(chunk);
    expect(tokens.map((token) => token.text).join('')).toBe(chunk);
  }
});

test('validateJsonText reports empty input without an error', () => {
  expect(validateJsonText('')).toEqual({ state: 'empty' });
  expect(validateJsonText('   \n  ')).toEqual({ state: 'empty' });
});

test('validateJsonText accepts objects, arrays, and scalars', () => {
  for (const value of ['{"a": 1}', '[1, 2]', '"text"', '42', 'true', 'null']) {
    expect(validateJsonText(value)).toEqual({ state: 'valid' });
  }
});

test('validateJsonText accepts quoted template expressions', () => {
  expect(validateJsonText('{"message": "{{ event.data.comment }}"}')).toEqual({ state: 'valid' });
});

test('validateJsonText reports broken input with a message', () => {
  for (const value of ['{"a": 1,}', '{"a": 1', '{a: 1}', '{"a": }', 'plain text']) {
    const result = validateJsonText(value);
    expect(result.state).toBe('invalid');
    if (result.state === 'invalid') expect(result.message.length).toBeGreaterThan(0);
  }
});

test('paste into an empty editor formats valid JSON', () => {
  expect(shouldFormatPastedJson({
    language: 'json',
    validateJson: true,
    formatJsonOnPaste: true,
    pastedText: '{"b":2,"a":1}',
    currentValue: '',
    selectionStart: 0,
    selectionEnd: 0,
  })).toBe(true);
});

test('paste replacing the whole document formats valid JSON', () => {
  const currentValue = '{"old":true}';
  expect(shouldFormatPastedJson({
    language: 'json',
    validateJson: true,
    formatJsonOnPaste: true,
    pastedText: '{"b":2}',
    currentValue,
    selectionStart: 0,
    selectionEnd: currentValue.length,
  })).toBe(true);
});

test('paste into the middle of a document keeps normal insertion', () => {
  const currentValue = '{"a": 1}';
  expect(shouldFormatPastedJson({
    language: 'json',
    validateJson: true,
    formatJsonOnPaste: true,
    pastedText: '{"b":2}',
    currentValue,
    selectionStart: 4,
    selectionEnd: 4,
  })).toBe(false);
});

test('paste of invalid JSON stays editable without formatting', () => {
  expect(shouldFormatPastedJson({
    language: 'json',
    validateJson: true,
    formatJsonOnPaste: true,
    pastedText: '{"b": }',
    currentValue: '',
    selectionStart: 0,
    selectionEnd: 0,
  })).toBe(false);
});

test('paste formatting is disabled for text editors or when opted out', () => {
  const base = {
    validateJson: true,
    formatJsonOnPaste: true,
    pastedText: '{"b":2}',
    currentValue: '',
    selectionStart: 0,
    selectionEnd: 0,
  };
  expect(shouldFormatPastedJson({ ...base, language: 'text' })).toBe(false);
  expect(shouldFormatPastedJson({ ...base, language: 'json', validateJson: false })).toBe(false);
  expect(shouldFormatPastedJson({ ...base, language: 'json', formatJsonOnPaste: false })).toBe(false);
});
