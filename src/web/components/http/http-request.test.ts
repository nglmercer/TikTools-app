import { expect, test } from 'bun:test';

import {
  buildHttpHeaders,
  getHeader,
  headerMapToText,
  normalizeBaseUrl,
  normalizeHttpMethod,
  parseHeadersText,
  readHttpBodyMode,
  readHttpConfig,
  suggestContentType,
} from './http-request.ts';

test('normalizeHttpMethod uppercases and falls back', () => {
  expect(normalizeHttpMethod('post')).toBe('POST');
  expect(normalizeHttpMethod('  get ')).toBe('GET');
  expect(normalizeHttpMethod(undefined)).toBe('POST');
  expect(normalizeHttpMethod('', 'GET')).toBe('GET');
  expect(normalizeHttpMethod(42, 'PUT')).toBe('PUT');
});

test('normalizeBaseUrl trims and strips trailing slashes', () => {
  expect(normalizeBaseUrl('  http://localhost:3000/  ')).toBe('http://localhost:3000');
  expect(normalizeBaseUrl('http://localhost:3000///')).toBe('http://localhost:3000');
  expect(normalizeBaseUrl('https://example.com/api/')).toBe('https://example.com/api');
  expect(normalizeBaseUrl('https://')).toBe('https://');
  expect(normalizeBaseUrl('not a url')).toBe('not a url');
});

test('header map and text representations round-trip', () => {
  const headers = { 'Content-Type': 'application/json', 'X-Token': 'abc' };
  const text = headerMapToText(headers);
  expect(text).toContain('Content-Type: application/json');
  expect(parseHeadersText(text)).toEqual(headers);
});

test('header text parsing skips blank and malformed lines', () => {
  expect(parseHeadersText('')).toEqual({});
  expect(parseHeadersText('no-separator\n: novalue\n  \nA: 1')).toEqual({ A: '1' });
  expect(headerMapToText(undefined)).toBe('');
  expect(headerMapToText('oops')).toBe('');
});

test('getHeader matches case-insensitively', () => {
  expect(getHeader({ 'Content-Type': 'text/plain' }, 'content-type')).toBe('text/plain');
  expect(getHeader({ 'X-A': '1' }, 'x-b')).toBeUndefined();
  expect(getHeader(undefined, 'x-a')).toBeUndefined();
});

test('buildHttpHeaders merges and replaces case-insensitively', () => {
  expect(buildHttpHeaders({ A: '1' }, { B: '2', C: undefined })).toEqual({ A: '1', B: '2' });
  expect(buildHttpHeaders({ 'content-type': 'text/html' }, { 'Content-Type': 'application/json' }))
    .toEqual({ 'Content-Type': 'application/json' });
  expect(buildHttpHeaders(undefined, {})).toEqual({});
});

test('readHttpBodyMode prefers explicit config, then content type, then default', () => {
  expect(readHttpBodyMode({ bodyMode: 'text' }, 'json')).toBe('text');
  expect(readHttpBodyMode({ bodyMode: 'json' }, 'text')).toBe('json');
  expect(readHttpBodyMode({ headers: { 'Content-Type': 'application/json' } }, 'text')).toBe('json');
  expect(readHttpBodyMode({ headers: { 'content-type': 'text/plain' } }, 'json')).toBe('text');
  expect(readHttpBodyMode({}, 'json')).toBe('json');
  expect(readHttpBodyMode({}, 'text')).toBe('text');
  expect(readHttpBodyMode({ bodyMode: 'xml' }, 'json')).toBe('json');
});

test('suggestContentType matches the body mode', () => {
  expect(suggestContentType('json')).toBe('application/json');
  expect(suggestContentType('text')).toBe('text/plain');
});

test('readHttpConfig types raw behavior and workflow configs alike', () => {
  expect(readHttpConfig({
    method: 'POST',
    url: 'https://example.com/hook',
    headers: { 'Content-Type': 'application/json' },
    body: '{"a":1}',
    timeoutMs: 5000,
    allowPrivateNetwork: 'true',
    emitResponseAs: 'evt.done',
  })).toEqual({
    method: 'POST',
    url: 'https://example.com/hook',
    headers: { 'Content-Type': 'application/json' },
    body: '{"a":1}',
    timeoutMs: 5000,
    responseType: undefined,
    redirect: undefined,
    allowPrivateNetwork: true,
    emitResponseAs: 'evt.done',
    bodyMode: undefined,
  });
  expect(readHttpConfig({ method: 7, timeoutMs: 'slow', allowPrivateNetwork: false })).toMatchObject({
    method: undefined,
    timeoutMs: undefined,
    allowPrivateNetwork: false,
  });
});
