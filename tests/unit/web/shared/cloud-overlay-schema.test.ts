import { testMessage } from '@/shared/i18n/test-message.ts';

const t = (key: string): string => testMessage('en', key);
import { describe, expect, it } from 'vitest';
import {
  addElement,
  emptyOverlay,
  needsLiveRead,
  readOverlayDocument,
  toWire,
  updateElement,
  validateOverlay,
  type OverlayDocument,
} from '../../../../src/shared/cloud-overlay-schema.ts';
import { defined } from '../../../shared/index.ts';

const asset = '3f2c9a1e-8b7d-4c6e-9a0b-1d2e3f4a5b6c';

describe('overlay schema', () => {
  it('accepts a plain overlay and reports problems in the Cloud codes', () => {
    const doc = emptyOverlay('gift', t('overlays.kinds.gift.name'));
    expect(validateOverlay(doc)).toEqual([]);
    expect(validateOverlay({ ...doc, canvas: { width: 100, height: 100 } })).toEqual(['invalidCanvas']);
    expect(validateOverlay({ ...doc, title: '   ' })).toEqual(['invalidTitle']);
  });

  it('keeps elements inside the canvas and the id rule', () => {
    let doc = addElement(emptyOverlay('chat', 'Chat'), 'text');
    const id = defined(doc.elements[0]).id;
    expect(validateOverlay(updateElement(doc, id, { x: 1900 }))).toContain('invalidGeometry');
    expect(validateOverlay(updateElement(doc, id, { opacity: 2 }))).toContain('invalidOpacity');
    expect(validateOverlay(updateElement(doc, id, { id: 'Bad Id' }))).toContain('invalidElementId');
    doc = addElement(doc, 'avatar');
    expect(validateOverlay(doc)).toEqual([]);
  });

  it('accepts only allow-listed fields and refuses avatar as a text field', () => {
    const doc = addElement(emptyOverlay('follow', 'Follow'), 'text');
    // A field outside the allow-list cannot be written in the type, so it arrives through JSON as it does from the Cloud.
    const withField = (field: string): OverlayDocument => {
      const elements = doc.elements.map((element) => ({ ...element, source: { kind: 'field', field } }));
      return JSON.parse(JSON.stringify({ ...doc, elements }));
    };
    expect(validateOverlay(withField('viewer.avatar'))).toContain('invalidSource');
    expect(validateOverlay(withField('viewer.name'))).toEqual([]);
  });

  it('requires a real asset id for art elements', () => {
    const doc = addElement(emptyOverlay('share', 'Share'), 'art');
    expect(validateOverlay(doc)).toContain('missingAsset');
    expect(validateOverlay(updateElement(doc, defined(doc.elements[0]).id, { assetId: asset }))).toEqual([]);
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

describe('readOverlayDocument', () => {
  it('returns a valid document and refuses everything else without throwing', () => {
    const doc = addElement(emptyOverlay('gift', t('cloudOverlays.kinds.gift')), 'text');
    expect(readOverlayDocument(doc)).toEqual(doc);
    expect(readOverlayDocument(null)).toBeNull();
    expect(readOverlayDocument('text')).toBeNull();
    expect(readOverlayDocument({})).toBeNull();
    expect(readOverlayDocument({ ...doc, canvas: null })).toBeNull();
    expect(readOverlayDocument({ ...doc, elements: [null] })).toBeNull();
    expect(readOverlayDocument({ ...doc, canvas: { width: 10, height: 10 } })).toBeNull();
  });
});
