const visibleElement = (element: Element | null): element is HTMLElement => {
  if (!(element instanceof HTMLElement) || !element.isConnected || element.closest('[hidden], [inert], [aria-hidden="true"]')) return false;
  for (let ancestor: HTMLElement | null = element; ancestor; ancestor = ancestor.parentElement) {
    const style = getComputedStyle(ancestor);
    if (style.display === 'none' || style.visibility === 'hidden' || Number.parseFloat(style.opacity) <= 0) return false;
  }
  return Array.from(element.getClientRects()).some((rect) => rect.width > 0 && rect.height > 0);
};

/** Poll for the visible element carrying `data-guide="name"`; null when it never shows up. */
export function waitForTarget(name: string, timeoutMs = 4000, root: ParentNode = document): Promise<HTMLElement | null> {
  const selector = `[data-guide="${CSS.escape(name)}"]`;
  return new Promise((resolve) => {
    const started = performance.now();
    const check = () => {
      const found = Array.from(root.querySelectorAll(selector)).find(visibleElement) ?? null;
      if (found || performance.now() - started >= timeoutMs) resolve(found);
      else requestAnimationFrame(check);
    };
    check();
  });
}

export function abortableSleep(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal.aborted) { reject(new DOMException('Showcase step interrupted.', 'AbortError')); return; }
    const timer = setTimeout(() => { signal.removeEventListener('abort', onAbort); resolve(); }, ms);
    const onAbort = () => { clearTimeout(timer); reject(new DOMException('Showcase step interrupted.', 'AbortError')); };
    signal.addEventListener('abort', onAbort, { once: true });
  });
}

export const prefersReducedMotion = () => {
  try { return matchMedia('(prefers-reduced-motion: reduce)').matches; } catch { return false; }
};
