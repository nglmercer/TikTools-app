import { expect, test } from 'bun:test';
import { designFromHash, normalizeDesign, parseDesign } from './design.ts';

test('portable designs reject invalid JSON, CSS and unknown properties', () => {
  expect(parseDesign('{')).toEqual({});
  expect(normalizeDesign(null)).toEqual({});
  expect(normalizeDesign({ background: 'url(evil)', accent: '#abc', radius: Infinity, token: 'secret' })).toEqual({});
  expect(normalizeDesign({ background: '#112233aa', radius: 100 })).toEqual({ background: '#112233aa', radius: 48 });
});

test('old designs still parse and new properties normalize', () => {
  expect(normalizeDesign({ background: '#16161d', textColor: '#f5f5f7', accent: '#22c55e', radius: 16, text: {} }))
    .toEqual({ text: {}, background: '#16161d', textColor: '#f5f5f7', accent: '#22c55e', radius: 16 });
  expect(normalizeDesign({
    borderColor: '#2a2a33', borderWidth: 99, shadow: false, opacity: 120,
    padding: -4, gap: 12, align: 'center', width: 100,
    avatar: { visible: false, size: 64, radius: 12, borderColor: '#ffffff', borderWidth: 3, evil: 1 },
    badge: { color: '#00dce8', fontSize: 14, fontWeight: 750, letterSpacing: 2 },
  })).toEqual({
    borderColor: '#2a2a33', borderWidth: 8, shadow: false, opacity: 100,
    padding: 0, gap: 12, align: 'center', width: 240,
    avatar: { visible: false, size: 64, radius: 12, borderColor: '#ffffff', borderWidth: 3 },
    badge: { color: '#00dce8', fontSize: 14, fontWeight: 800, letterSpacing: 2 },
  });
});

test('design normalization keeps a clean hidden-field list', () => {
  expect(normalizeDesign({ hiddenText: ['title', 'name', 'title', 'evil', 5] }).hiddenText)
    .toEqual(['title', 'name']);
  expect(normalizeDesign({ hiddenText: [] })).not.toHaveProperty('hiddenText');
  expect(normalizeDesign({ hiddenText: 'title' })).not.toHaveProperty('hiddenText');
  expect(normalizeDesign({})).not.toHaveProperty('hiddenText');
});

test('design normalization drops invalid enums, colors, and empty groups', () => {
  expect(normalizeDesign({ align: 'diagonal', borderColor: 'red', shadow: 'yes', opacity: NaN })).toEqual({});
  expect(normalizeDesign({ avatar: { size: 'big' }, badge: 'none' })).toEqual({});
  expect(normalizeDesign({ avatar: [], badge: { visible: 'yes' } })).toEqual({});
});

test('OBS parses encoded designs without mixing in the authentication token', () => {
  const design = { background: '#112233', accent: '#ff00ff', radius: 0 };
  expect(designFromHash(`#token=secret&design=${encodeURIComponent(JSON.stringify(design))}`)).toEqual(design);
  expect(designFromHash('#token=secret')).toEqual({});
});
