import type { DemoLocale } from '../scenarios.ts';
import type { ShowcaseStage } from './director.ts';
import { showcaseCopy } from './copy.ts';
import {
  clickPressMs, clickSettleMs, cursorHideMs, cursorTravel, easeOut, glideEasing, layerData,
  rippleMs, spotlightFollowMs, spotlightSettleMs,
} from './constants.ts';
import './showcase.css';

export interface StageOptions { locale: DemoLocale; reducedMotion: boolean }

const SVG_NS = 'http://www.w3.org/2000/svg';

function cursorElement() {
  const svg = document.createElementNS(SVG_NS, 'svg');
  svg.setAttribute('viewBox', '0 0 22 22');
  svg.setAttribute('class', 'showcase__cursor');
  const path = document.createElementNS(SVG_NS, 'path');
  path.setAttribute('d', 'M3 2l15 8.2-6.6 1.7L8.6 18.5z');
  svg.append(path);
  return svg;
}

/** Spotlight, tooltip and cursor drawn above the real demo. It reads layout; it never changes the app. */
export function createShowcaseStage(options: StageOptions): ShowcaseStage & { dispose(): void } {
  const root = document.createElement('div');
  root.className = 'showcase';
  root.setAttribute('aria-hidden', 'true');
  const spot = document.createElement('div'); spot.className = 'showcase__spot';
  const tip = document.createElement('div'); tip.className = 'showcase__tip';
  const cursor = cursorElement();
  const ripple = document.createElement('div'); ripple.className = 'showcase__ripple';
  root.append(spot, tip, ripple, cursor);
  document.body.append(root);
  if (options.reducedMotion) root.dataset[layerData.still] = 'true';

  let focused: HTMLElement | null = null;
  let pointer = { x: innerWidth * 0.7, y: innerHeight * 0.85 };
  let observer: ResizeObserver | undefined;
  let frame = 0;
  let hideTimer: ReturnType<typeof setTimeout> | undefined;

  let side: 'top' | 'bottom' = 'bottom';
  const place = () => {
    if (!focused?.isConnected) return;
    const rect = focused.getBoundingClientRect();
    const pad = 6;
    spot.style.transform = `translate(${rect.left - pad}px, ${rect.top - pad}px)`;
    spot.style.width = `${rect.width + pad * 2}px`;
    spot.style.height = `${rect.height + pad * 2}px`;
    const tw = tip.offsetWidth, th = tip.offsetHeight;
    const x = Math.min(Math.max(8, rect.left + rect.width / 2 - tw / 2), innerWidth - tw - 8);
    let y = rect.bottom + pad + 12;
    // Large targets get the tooltip inside their lower edge instead of off screen.
    if (y + th > innerHeight - 8) y = rect.top - th - pad - 12;
    if (y < 8 || rect.height > innerHeight * 0.7) y = side === 'top' ? Math.max(8, rect.top) + 14 : Math.max(8, Math.min(rect.bottom, innerHeight) - th - 18);
    tip.style.transform = `translate(${x}px, ${y}px)`;
  };
  // Layout settles during the spotlight transition (views fade, rows slide), so keep following it briefly.
  const follow = (until: number) => {
    cancelAnimationFrame(frame);
    const step = () => { place(); if (performance.now() < until) frame = requestAnimationFrame(step); };
    step();
  };
  const settle = (ms: number) => options.reducedMotion || ms <= 0 ? Promise.resolve() : new Promise<void>((resolve) => { setTimeout(resolve, ms); });
  const onResize = () => place();
  addEventListener('resize', onResize);

  async function travel(x: number, y: number) {
    const from = pointer;
    pointer = { x, y };
    const distance = Math.hypot(x - from.x, y - from.y);
    const duration = options.reducedMotion ? 0 : Math.min(cursorTravel.maxMs, cursorTravel.baseMs + distance * cursorTravel.perPx);
    const frames = [{ transform: `translate(${from.x - 4}px, ${from.y - 2}px)` }, { transform: `translate(${x - 4}px, ${y - 2}px)` }];
    const animation = cursor.animate(frames, { duration, easing: glideEasing, fill: 'forwards' });
    cursor.dataset[layerData.visible] = 'true';
    await animation.finished.catch(() => undefined);
  }

  return {
    async focus(element, step) {
      focused = element;
      side = step.tip;
      tip.textContent = showcaseCopy[options.locale][step.id];
      root.dataset[layerData.on] = 'true';
      observer?.disconnect();
      if (typeof ResizeObserver === 'function') { observer = new ResizeObserver(place); observer.observe(element); }
      follow(performance.now() + spotlightSettleMs + spotlightFollowMs);
      await settle(spotlightSettleMs);
    },
    async pointAt(element) {
      const rect = element.getBoundingClientRect();
      await travel(rect.left + Math.min(rect.width * 0.5, 90), rect.top + rect.height * 0.55);
    },
    async click() {
      ripple.style.transform = `translate(${pointer.x}px, ${pointer.y}px)`;
      if (!options.reducedMotion) {
        cursor.animate([{ scale: '1' }, { scale: '0.82' }, { scale: '1' }], { duration: clickPressMs, easing: easeOut });
        ripple.animate([{ opacity: 0.9, scale: '0.3' }, { opacity: 0, scale: '1.8' }], { duration: rippleMs, easing: easeOut });
      }
      await settle(clickSettleMs);
      // The cursor only exists to show a click; it leaves once the result is visible.
      clearTimeout(hideTimer);
      hideTimer = setTimeout(() => { delete cursor.dataset[layerData.visible]; }, cursorHideMs);
    },
    release() { delete root.dataset[layerData.on]; },
    clear() {
      clearTimeout(hideTimer);
      delete root.dataset[layerData.on];
      delete cursor.dataset[layerData.visible];
      focused = null;
      observer?.disconnect();
      cancelAnimationFrame(frame);
    },
    dispose() {
      clearTimeout(hideTimer);
      removeEventListener('resize', onResize);
      cancelAnimationFrame(frame);
      observer?.disconnect();
      root.remove();
    },
  };
}
