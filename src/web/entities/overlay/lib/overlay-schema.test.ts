import { describe, expect, it } from 'vitest';
import { addElement, emptyOverlay, needsLiveRead, toWire, updateElement, validateOverlay } from './overlay-schema.ts';

const asset = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';

describe('overlay schema', () => {
  it('accepts a plain overlay and reports problems in the Cloud codes', () => {
    const doc = emptyOverlay('gift', 'Gift alert');
    expect(validateOverlay(doc)).toEqual([]);
    expect(validateOverlay({ ...doc, canvas: { width: 100, height: 100 } })).toEqual(['invalidCanvas']);
    expect(validateOverlay({ ...doc, title: '   ' })).toEqual(['invalidTitle']);
  });

  it('keeps elements inside the canvas and the id rule', () => {
    let doc = addElement(emptyOverlay('chat', 'Chat'), 'text');
    const id = doc.elements[0]!.id;
    expect(validateOverlay(updateElement(doc, id, { x: 1900 }))).toContain('invalidGeometry');
    expect(validateOverlay(updateElement(doc, id, { opacity: 2 }))).toContain('invalidOpacity');
    expect(validateOverlay(updateElement(doc, id, { id: 'Bad Id' }))).toContain('invalidElementId');
    doc = addElement(doc, 'avatar');
    expect(validateOverlay(doc)).toEqual([]);
  });

  it('accepts only allow-listed fields and refuses avatar as a text field', () => {
    const doc = addElement(emptyOverlay('follow', 'Follow'), 'text');
    const text = doc.elements[0] as { source: unknown };
    text.source = { kind: 'field', field: 'viewer.avatar' };
    expect(validateOverlay(doc)).toContain('invalidSource');
    text.source = { kind: 'field', field: 'viewer.name' };
    expect(validateOverlay(doc)).toEqual([]);
  });

  it('requires a real asset id for art elements', () => {
    const doc = addElement(emptyOverlay('share', 'Share'), 'art');
    expect(validateOverlay(doc)).toContain('missingAsset');
    expect(validateOverlay(updateElement(doc, doc.elements[0]!.id, { assetId: asset }))).toEqual([]);
  });

  it('declares the live-read capability only when a value comes from the stream', () => {
    const plain = addElement(emptyOverlay('likes', 'Likes'), 'text');
    expect(needsLiveRead(plain)).toBe(false);
    expect(toWire(plain)).not.toHaveProperty('capabilities');
    const live = addElement(plain, 'avatar');
    expect(needsLiveRead(live)).toBe(true);
    expect(toWire(live).capabilities).toEqual(['live.read']);
  });
});
