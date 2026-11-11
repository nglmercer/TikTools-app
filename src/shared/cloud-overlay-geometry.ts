import type { OverlayDocument, OverlayElement } from './cloud-overlay-schema.ts';

/**
 * Pure layout rules shared by the editor preview and the OBS widget, so one valid
 * overlay document occupies the same place in both. Only geometry lives here:
 * placeholders, sample text, live values and styling stay with each renderer.
 * This module must not import anything from the Vue application.
 */

export interface OverlayCanvas { width: number; height: number }

/** CSS-ready layout of one element, as percentages of the canvas. */
export interface OverlayBoxStyle {
  left: string;
  top: string;
  width: string;
  height: string;
  opacity: string;
}

/** A canvas with positive, finite dimensions; anything else cannot be laid out. */
export function hasValidCanvas(canvas: OverlayCanvas | null | undefined): canvas is OverlayCanvas {
  return !!canvas && Number.isFinite(canvas.width) && Number.isFinite(canvas.height)
    && canvas.width > 0 && canvas.height > 0;
}

/** `width / height` for the CSS `aspect-ratio` of the frame, or `null` for an unusable canvas. */
export function overlayAspectRatio(canvas: OverlayCanvas | null | undefined): string | null {
  return hasValidCanvas(canvas) ? `${canvas.width} / ${canvas.height}` : null;
}

/** Opacity from 0 to 1; a missing or non-numeric value is fully opaque. */
export function overlayOpacity(opacity: number | undefined): number {
  if (typeof opacity !== 'number' || !Number.isFinite(opacity)) return 1;
  return Math.min(1, Math.max(0, opacity));
}

function percent(value: number, total: number): string {
  return `${(value / total) * 100}%`;
}

/**
 * Position and size as percentages of the canvas, so the layout scales to any source
 * size. Returns `null` when the canvas has invalid dimensions rather than `NaN%` values.
 */
export function overlayBoxStyle(
  canvas: OverlayCanvas | null | undefined,
  element: Pick<OverlayElement, 'x' | 'y' | 'width' | 'height' | 'opacity'>,
): OverlayBoxStyle | null {
  if (!hasValidCanvas(canvas)) return null;
  return {
    left: percent(element.x, canvas.width),
    top: percent(element.y, canvas.height),
    width: percent(element.width, canvas.width),
    height: percent(element.height, canvas.height),
    opacity: String(overlayOpacity(element.opacity)),
  };
}

/** Frame style for a document: its aspect ratio, or nothing for a missing document or invalid canvas. */
export function overlayFrameStyle(document: Pick<OverlayDocument, 'canvas'> | null | undefined): { aspectRatio: string } | Record<string, never> {
  const aspectRatio = overlayAspectRatio(document?.canvas);
  return aspectRatio ? { aspectRatio } : {};
}
