import type { Directive } from 'vue';
import './spotlight.css';

/**
 * State-driven, driver.js-style spotlight for the browser demo.
 * The caller marks an element as `active` (connection succeeded, action failed)
 * and the rest of the page is dimmed until the state changes. No hover is
 * required, so it also works on touch devices.
 */
const OVERLAY_ID = 'app-spotlight';
const activeElements = new Set<HTMLElement>();
const dismissed = new WeakSet<HTMLElement>();
let overlay: HTMLDivElement | null = null;
let target: HTMLElement | null = null;
let listening = false;
let observer: MutationObserver | null = null;
let blocked = false;

function enabled(): boolean {
  return document.documentElement.hasAttribute('data-demo')
    && !document.querySelector('dialog[open], .driver-popover');
}

function visible(element: HTMLElement): boolean {
  if (!element.isConnected) return false;
  const rect = element.getBoundingClientRect();
  return rect.width > 0 && rect.height > 0 && getComputedStyle(element).visibility !== 'hidden';
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

function hide(): void {
  target = null;
  overlay?.classList.remove('spotlight-overlay--visible');
  if (overlay) overlay.hidden = true;
  if (listening) {
    window.removeEventListener('scroll', reposition, true);
    window.removeEventListener('resize', reposition);
    listening = false;
  }
}

function watchBlockers(): void {
  if (observer) return;
  observer = new MutationObserver(() => {
    const now = Boolean(document.querySelector('dialog[open], .driver-popover'));
    if (now !== blocked) { blocked = now; render(); }
  });
  observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['open', 'class'] });
}

function render(): void {
  const element = [...activeElements].reverse().find((candidate) => !dismissed.has(candidate) && visible(candidate));
  if (!element || !enabled()) { hide(); return; }
  const node = ensureOverlay();
  if (target !== element) {
    target = element;
    node.hidden = false;
    requestAnimationFrame(() => node.classList.add('spotlight-overlay--visible'));
  }
  place(element);
  if (!listening) {
    window.addEventListener('scroll', reposition, true);
    window.addEventListener('resize', reposition);
    listening = true;
  }
}

function setActive(element: HTMLElement, active: boolean): void {
  if (active) {
    if (!activeElements.has(element)) dismissed.delete(element);
    activeElements.add(element);
  } else activeElements.delete(element);
  watchBlockers();
  watchDismissal();
  render();
}

/** Called when the highlighted action is executed or the reader dismisses it. */
export function dismissSpotlight(): void {
  if (target) dismissed.add(target);
  hide();
}

function onEscape(event: KeyboardEvent): void {
  if (event.key === 'Escape' && target) dismissSpotlight();
}

function onOutsidePointer(event: PointerEvent): void {
  if (!target || (event.target instanceof Node && target.contains(event.target))) return;
  dismissSpotlight();
}

let globalListening = false;
function watchDismissal(): void {
  if (!globalListening && activeElements.size) {
    window.addEventListener('keydown', onEscape);
    document.addEventListener('pointerdown', onOutsidePointer, true);
    globalListening = true;
  } else if (globalListening && !activeElements.size) {
    window.removeEventListener('keydown', onEscape);
    document.removeEventListener('pointerdown', onOutsidePointer, true);
    globalListening = false;
  }
}

export const vSpotlight: Directive<HTMLElement, boolean> = {
  mounted(element, binding) { setActive(element, binding.value === true); },
  updated(element, binding) {
    if (binding.value !== binding.oldValue) setActive(element, binding.value === true);
  },
  unmounted(element) { setActive(element, false); },
};
