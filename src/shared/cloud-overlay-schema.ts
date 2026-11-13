/**
 * Overlay schema version 1, as the Cloud checks it. The editor uses these rules to
 * show problems before a save; the Cloud remains the authority and refuses any
 * document that does not pass its own check.
 */
export const OVERLAY_KINDS = ['follow', 'gift', 'chat', 'share', 'subscribe', 'likes', 'ranking'] as const;
export type OverlayKind = (typeof OVERLAY_KINDS)[number];
export const OVERLAY_FIELDS = [
  'viewer.name',
  'viewer.handle',
  'gift.name',
  'gift.count',
  'gift.diamonds',
  'chat.message',
  'points.total',
] as const;
export type OverlayField = (typeof OVERLAY_FIELDS)[number];

export const MAX_ELEMENTS = 64;
export const CANVAS = { minWidth: 320, maxWidth: 3840, minHeight: 180, maxHeight: 2160 } as const;
const MAX_TEXT = 200;

export type TextSource = { kind: 'static'; text: string } | { kind: 'field'; field: OverlayField };

interface Box { id: string; x: number; y: number; width: number; height: number; opacity?: number }
export interface TextElement extends Box { type: 'text'; source: TextSource }
export interface AvatarElement extends Box { type: 'avatar' }
export interface ArtElement extends Box { type: 'art'; assetId: string }
export type OverlayElement = TextElement | AvatarElement | ArtElement;

export interface OverlayDocument {
  overlayVersion: 1;
  kind: OverlayKind;
  title: string;
  canvas: { width: number; height: number };
  elements: OverlayElement[];
  capabilities?: 'live.read'[];
}

export type OverlayProblem =
  | 'invalidTitle' | 'invalidCanvas' | 'tooManyElements' | 'invalidElementId' | 'duplicateElementId'
  | 'invalidGeometry' | 'invalidOpacity' | 'invalidSource' | 'missingAsset' | 'invalidElementType';

/** Control characters other than newline are refused. */
function hasControl(text: string): boolean {
  return [...text].some((character) => {
    const code = character.codePointAt(0) ?? 0;
    return (code < 0x20 && code !== 0x0a) || code === 0x7f;
  });
}

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const ELEMENT_ID = /^[a-z0-9_-]{1,64}$/;

/** Returns every problem found, in the Cloud's terms. An empty list means the editor accepts it. */
export function validateOverlay(document: OverlayDocument): OverlayProblem[] {
  const problems = new Set<OverlayProblem>();
  if (document.title.trim().length === 0 || document.title.length > 120) problems.add('invalidTitle');
  const { width, height } = document.canvas;
  if (!Number.isInteger(width) || !Number.isInteger(height)
    || width < CANVAS.minWidth || width > CANVAS.maxWidth || height < CANVAS.minHeight || height > CANVAS.maxHeight) {
    problems.add('invalidCanvas');
  }
  if (document.elements.length > MAX_ELEMENTS) problems.add('tooManyElements');
  const ids = new Set<string>();
  for (const element of document.elements) {
    if (!ELEMENT_ID.test(element.id)) problems.add('invalidElementId');
    if (ids.has(element.id)) problems.add('duplicateElementId');
    ids.add(element.id);
    const fits = Number.isInteger(element.x) && Number.isInteger(element.y)
      && Number.isInteger(element.width) && Number.isInteger(element.height)
      && element.x >= 0 && element.y >= 0 && element.width >= 1 && element.height >= 1
      && element.x + element.width <= width && element.y + element.height <= height;
    if (!fits) problems.add('invalidGeometry');
    if (element.opacity !== undefined && (element.opacity < 0 || element.opacity > 1)) problems.add('invalidOpacity');
    if (element.type === 'text') {
      if (element.source.kind === 'static') {
        if (element.source.text.length > MAX_TEXT || hasControl(element.source.text)) {
          problems.add('invalidSource');
        }
      } else if (!(OVERLAY_FIELDS as readonly string[]).includes(element.source.field)) {
        problems.add('invalidSource');
      }
    } else if (element.type === 'art') {
      if (!UUID.test(element.assetId)) problems.add('missingAsset');
    }
  }
  return [...problems];
}

export function emptyOverlay(kind: OverlayKind, title: string): OverlayDocument {
  return { overlayVersion: 1, kind, title: title.trim(), canvas: { width: 1920, height: 1080 }, elements: [] };
}

/** Next free element id derived from the element type, for example `text-3`. */
export function nextElementId(document: OverlayDocument, type: OverlayElement['type']): string {
  const used = new Set(document.elements.map((element) => element.id));
  for (let index = 1; index <= MAX_ELEMENTS + 1; index += 1) {
    const candidate = `${type}-${index}`;
    if (!used.has(candidate)) return candidate;
  }
  return `${type}-${Date.now()}`;
}

export function addElement(document: OverlayDocument, type: OverlayElement['type']): OverlayDocument {
  const id = nextElementId(document, type);
  const base = { id, x: 0, y: 0, width: 240, height: 80 };
  const element: OverlayElement = type === 'text'
    ? { ...base, type, source: { kind: 'static', text: 'Text' } }
    : type === 'avatar'
      ? { ...base, type, width: 96, height: 96 }
      : { ...base, type, assetId: '' };
  return { ...document, elements: [...document.elements, element] };
}

export function removeElement(document: OverlayDocument, id: string): OverlayDocument {
  return { ...document, elements: document.elements.filter((element) => element.id !== id) };
}

export function updateElement(
  document: OverlayDocument,
  id: string,
  patch: Partial<Box & { assetId: string }>,
): OverlayDocument {
  return {
    ...document,
    elements: document.elements.map((element) =>
      element.id === id ? ({ ...element, ...patch } as OverlayElement) : element,
    ),
  };
}

/** Whether the document reads live values, so the Cloud requires the `live.read` capability. */
export function needsLiveRead(document: OverlayDocument): boolean {
  return document.elements.some((element) =>
    element.type === 'avatar' || (element.type === 'text' && element.source.kind === 'field'));
}

/** The document as the Cloud expects it, with the capability declared when required. */
export function toWire(document: OverlayDocument): OverlayDocument {
  const wire: OverlayDocument = { ...document };
  delete wire.capabilities;
  return needsLiveRead(document) ? { ...wire, capabilities: ['live.read'] } : wire;
}

/**
 * Reads untrusted JSON (for example the OBS URL fragment) as an overlay document.
 * Returns `null` for anything that is not an object, is structurally incomplete or
 * fails `validateOverlay`; a partial layout is never returned.
 */
export function readOverlayDocument(value: unknown): OverlayDocument | null {
  if (value === null || typeof value !== 'object' || Array.isArray(value)) return null;
  const candidate = value as Partial<OverlayDocument>;
  if (candidate.overlayVersion !== 1 || !(OVERLAY_KINDS as readonly unknown[]).includes(candidate.kind)
    || typeof candidate.title !== 'string'
    || !candidate.canvas || typeof candidate.canvas !== 'object' || !Array.isArray(candidate.elements)) {
    return null;
  }
  if (candidate.capabilities !== undefined && (!Array.isArray(candidate.capabilities)
    || candidate.capabilities.length > 1 || candidate.capabilities.some((capability) => capability !== 'live.read'))) return null;
  for (const element of candidate.elements) {
    if (!element || typeof element !== 'object' || typeof element.id !== 'string'
      || !['text', 'avatar', 'art'].includes(element.type)
      || element.opacity !== undefined && (typeof element.opacity !== 'number' || !Number.isFinite(element.opacity))) return null;
    if (element.type === 'text') {
      if (!element.source || typeof element.source !== 'object'
        || (element.source.kind !== 'static' && element.source.kind !== 'field')
        || element.source.kind === 'static' && typeof element.source.text !== 'string'
        || element.source.kind === 'field' && typeof element.source.field !== 'string') return null;
    }
    if (element.type === 'art' && typeof element.assetId !== 'string') return null;
  }
  try {
    return validateOverlay(candidate as OverlayDocument).length === 0 ? (candidate as OverlayDocument) : null;
  } catch {
    return null;
  }
}
