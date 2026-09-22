import { expect, test } from 'bun:test';
import { designFromHash, normalizeDesign, parseDesign } from './design.ts';

test('portable designs reject invalid JSON, CSS and unknown properties', () => {
  expect(parseDesign('{')).toEqual({});
  expect(normalizeDesign(null)).toEqual({});
  expect(normalizeDesign({ background: 'url(evil)', accent: '#abc', radius: Infinity, token: 'secret' })).toEqual({});
  expect(normalizeDesign({ background: '#112233aa', radius: 100 })).toEqual({ background: '#112233aa', radius: 48 });
});

test('OBS parses encoded designs without mixing in the authentication token', () => {
  const design = { background: '#112233', accent: '#ff00ff', radius: 0 };
  expect(designFromHash(`#token=secret&design=${encodeURIComponent(JSON.stringify(design))}`)).toEqual(design);
  expect(designFromHash('#token=secret')).toEqual({});
});
