/** DOM contract shared by tours, the demo showcase and specs: how guide targets are addressed and judged visible. */
export const GUIDE_TARGET_ATTR = 'data-guide';
export const GUIDE_SCOPE_ATTR = 'data-guide-scope';

const quote = (value: string) => value.replace(/["\\]/g, '\\$&');
export const guideTargetSelector = (name: string): string => `[${GUIDE_TARGET_ATTR}="${quote(name)}"]`;
export const guideScopeSelector = (name: string): string => `[${GUIDE_SCOPE_ATTR}="${quote(name)}"]`;

/** Scoped `querySelector` that treats an invalid selector as "not found". */
export function queryGuide(scope: Element | Document, selector: string): Element | null {
  try { return scope instanceof Element && scope.matches(selector) ? scope : scope.querySelector(selector); }
  catch { return null; }
}

/** Laid out, not hidden, not inert: the element can be pointed at. */
export function isRendered(element: Element | null): element is HTMLElement {
  if (!(element instanceof HTMLElement) || !element.isConnected || element.closest('[hidden], [inert], [aria-hidden="true"]')) return false;
  for (let ancestor: HTMLElement | null = element; ancestor; ancestor = ancestor.parentElement) {
    const style = getComputedStyle(ancestor);
    if (style.display === 'none' || style.visibility === 'hidden' || style.visibility === 'collapse' || Number.parseFloat(style.opacity) <= 0) return false;
  }
  return Array.from(element.getClientRects()).some((rect) => rect.width > 0 && rect.height > 0);
}

/** Rendered and interactive: a guide can highlight it and the user can act on it. */
export function isVisible(element: Element | null): element is HTMLElement {
  return isRendered(element) && !element.matches(':disabled, [aria-disabled="true"]');
}
