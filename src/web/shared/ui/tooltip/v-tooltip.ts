import type { Directive } from 'vue';
import { computeAnchoredPosition } from '@/shared/lib/anchored-position.ts';
import './tooltip.css';

const SHOW_DELAY_MS = 500;
const TOOLTIP_ID = 'app-tooltip';

interface TooltipState {
  text: string;
  timer?: ReturnType<typeof setTimeout>;
  cleanup: () => void;
}

const states = new WeakMap<HTMLElement, TooltipState>();
let element: HTMLDivElement | null = null;
let owner: HTMLElement | null = null;

function tooltipElement(): HTMLDivElement {
  if (element?.isConnected) return element;
  element = document.createElement('div');
  element.id = TOOLTIP_ID;
  element.className = 'base-tooltip';
  element.setAttribute('role', 'tooltip');
  element.hidden = true;
  document.body.append(element);
  return element;
}

function describes(target: HTMLElement, text: string): boolean {
  return target.getAttribute('aria-label') !== text;
}

function hide(target: HTMLElement): void {
  const state = states.get(target);
  clearTimeout(state?.timer);
  if (owner !== target || !element) return;
  element.hidden = true;
  if (target.getAttribute('aria-describedby') === TOOLTIP_ID) target.removeAttribute('aria-describedby');
  owner = null;
}

function show(target: HTMLElement): void {
  const state = states.get(target);
  if (!state?.text || !target.isConnected) return;
  if (owner && owner !== target) hide(owner);
  const tooltip = tooltipElement();
  // A modal dialog sits in the top layer, above anything appended to the body.
  const host = target.closest('dialog[open]') ?? document.body;
  if (tooltip.parentElement !== host) host.append(tooltip);
  tooltip.textContent = state.text;
  tooltip.hidden = false;
  const position = computeAnchoredPosition(
    target.getBoundingClientRect(),
    { width: tooltip.offsetWidth, height: tooltip.offsetHeight },
    { width: window.innerWidth, height: window.innerHeight },
    'bottom-start',
    4,
  );
  tooltip.style.top = `${position.top}px`;
  tooltip.style.left = `${position.left}px`;
  if (describes(target, state.text)) target.setAttribute('aria-describedby', TOOLTIP_ID);
  owner = target;
}

function schedule(target: HTMLElement): void {
  const state = states.get(target);
  if (!state?.text) return;
  clearTimeout(state.timer);
  state.timer = setTimeout(() => show(target), SHOW_DELAY_MS);
}

export const vTooltip: Directive<HTMLElement, string | undefined> = {
  mounted(target, binding) {
    const onEnter = (): void => schedule(target);
    const onLeave = (): void => hide(target);
    const onKeydown = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') hide(target);
    };
    target.addEventListener('pointerenter', onEnter);
    target.addEventListener('focusin', onEnter);
    target.addEventListener('pointerleave', onLeave);
    target.addEventListener('pointerdown', onLeave);
    target.addEventListener('focusout', onLeave);
    target.addEventListener('keydown', onKeydown);
    states.set(target, {
      text: binding.value ?? '',
      cleanup: () => {
        target.removeEventListener('pointerenter', onEnter);
        target.removeEventListener('focusin', onEnter);
        target.removeEventListener('pointerleave', onLeave);
        target.removeEventListener('pointerdown', onLeave);
        target.removeEventListener('focusout', onLeave);
        target.removeEventListener('keydown', onKeydown);
      },
    });
  },
  updated(target, binding) {
    const state = states.get(target);
    if (!state || state.text === binding.value) return;
    state.text = binding.value ?? '';
    if (owner !== target) return;
    if (state.text) show(target);
    else hide(target);
  },
  beforeUnmount(target) {
    hide(target);
    states.get(target)?.cleanup();
    states.delete(target);
  },
};
