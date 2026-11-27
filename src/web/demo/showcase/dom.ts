import { guideTargetSelector, isRendered } from '@/features/guided-help/model/guide-dom.ts';
import { targetWaitMs } from './constants.ts';

const interruptedMessage = 'Showcase step interrupted.';
/** The rejection a superseded, paused or disposed step carries. */
export const abortError = (): DOMException => new DOMException(interruptedMessage, 'AbortError');
export const isAbortError = (error: unknown): boolean => error instanceof DOMException && error.name === 'AbortError';

/** Poll for the rendered element carrying `data-guide="name"`; null when it never shows up. */
export function waitForTarget(name: string, timeoutMs = targetWaitMs, root: ParentNode = document): Promise<HTMLElement | null> {
  const selector = guideTargetSelector(name);
  return new Promise((resolve) => {
    const started = performance.now();
    const check = () => {
      const found = Array.from(root.querySelectorAll(selector)).find(isRendered) ?? null;
      if (found || performance.now() - started >= timeoutMs) resolve(found);
      else requestAnimationFrame(check);
    };
    check();
  });
}

export function abortableSleep(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    if (signal.aborted) { reject(abortError()); return; }
    const timer = setTimeout(() => { signal.removeEventListener('abort', onAbort); resolve(); }, ms);
    const onAbort = () => { clearTimeout(timer); reject(abortError()); };
    signal.addEventListener('abort', onAbort, { once: true });
  });
}

export const prefersReducedMotion = () => {
  try { return matchMedia('(prefers-reduced-motion: reduce)').matches; } catch { return false; }
};
