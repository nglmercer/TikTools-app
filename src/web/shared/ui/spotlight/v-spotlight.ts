import type { Directive } from 'vue';
import './spotlight.css';

/**
 * Driver.js-style spotlight for short-lived hovers in the browser demo.
 * A single fixed cutout darkens the rest of the page with a large box-shadow;
 * no tour engine, popover or navigation state is involved. The desktop app
 * (without `data-demo`) and an active guided tour are never affected.
 */
const OVERLAY_ID = 'app-spotlight';
const listeners = new WeakMap<HTMLElement, () => void>();
let overlay: HTMLDivElement | null = null;
let target: HTMLElement | null = null;

function enabled(): boolean {
  return document.documentElement.hasAttribute('data-demo')
    && !document.querySelector('dialog[open], .driver-popover');
}

function ensureOverlay(): HTMLDivElement {
  if (overlay?.isConnected) return overlay;
  overlay = document.createElement('div');
  overlay.id = OVERLAY_ID;
  overlay.className = 'spotlight-overlay';
  overlay.hidden = true;
  document.body.append(overlay);
  return overlay;
}

function place(element: HTMLElement): void {
  if (!overlay) return;
  const rect = element.getBoundingClientRect();
  const padding = 3;
  overlay.style.left = `${rect.left - padding}px`;
  overlay.style.top = `${rect.top - padding}px`;
  overlay.style.width = `${rect.width + padding * 2}px`;
  overlay.style.height = `${rect.height + padding * 2}px`;
}

function reposition(): void {
  if (target) place(target);
}

function show(element: HTMLElement): void {
  if (!enabled()) return;
  target = element;
  const node = ensureOverlay();
  place(element);
  node.hidden = false;
  requestAnimationFrame(() => node.classList.add('spotlight-overlay--visible'));
  window.addEventListener('scroll', reposition, true);
  window.addEventListener('resize', reposition);
}

function hide(element: HTMLElement): void {
  if (target !== element) return;
  target = null;
  overlay?.classList.remove('spotlight-overlay--visible');
  if (overlay) overlay.hidden = true;
  window.removeEventListener('scroll', reposition, true);
  window.removeEventListener('resize', reposition);
}

export const vSpotlight: Directive<HTMLElement> = {
  mounted(element) {
    const enter = (): void => show(element);
    const leave = (): void => hide(element);
    element.addEventListener('pointerenter', enter);
    element.addEventListener('focusin', enter);
    element.addEventListener('pointerleave', leave);
    element.addEventListener('focusout', leave);
    const dispose = (): void => {
      element.removeEventListener('pointerenter', enter);
      element.removeEventListener('focusin', enter);
      element.removeEventListener('pointerleave', leave);
      element.removeEventListener('focusout', leave);
      hide(element);
    };
    listeners.set(element, dispose);
  },
  unmounted(element) {
    listeners.get(element)?.();
    listeners.delete(element);
  },
};
