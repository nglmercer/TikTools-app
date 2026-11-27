import { nextTick } from 'vue';
import type { Router } from 'vue-router';
import { runGuideAction } from '@/features/guided-help/model/guide-actions.ts';
import type { DemoLocale } from '../scenarios.ts';
import { createShowcaseDirector, type ShowcaseDirector, type ShowcaseState } from './director.ts';
import { createShowcaseStage } from './stage.ts';
import { abortableSleep, prefersReducedMotion, waitForTarget } from './dom.ts';
import { showcaseSteps } from './script.ts';
import { demoRootData } from '../contract.ts';
import { viewEnterMs } from './constants.ts';

export interface ShowcaseHandle extends ShowcaseDirector {
  readonly reducedMotion: boolean;
}

export interface ShowcaseMountOptions {
  router: Router;
  stepEvent(): Promise<void>;
  locale: DemoLocale;
  /** Recording mode plays once and flags the end on `<html data-showcase-done>`. */
  record: boolean;
  /** Speed-up factor; every wait and animation is divided by it. */
  pace: number;
  /** Cross-fade views with the View Transitions API. */
  viewTransitions: boolean;
  changed(state: ShowcaseState): void;
}

export function mountShowcase(options: ShowcaseMountOptions): ShowcaseHandle {
  const reducedMotion = prefersReducedMotion();
  const stage = createShowcaseStage({ locale: options.locale, reducedMotion, pace: options.pace });
  const director = createShowcaseDirector(showcaseSteps, {
    stage,
    async navigate(route) {
      if (options.router.currentRoute.value.fullPath === route) return;
      const go = async () => { await options.router.push(route); await nextTick(); };
      // Progressive enhancement: browsers without View Transitions switch views instantly.
      if (options.viewTransitions && !reducedMotion && typeof document.startViewTransition === 'function') await document.startViewTransition(go).updateCallbackDone;
      else {
        // Framed demos are scaled inside the docs page, where View Transitions crash headless Chromium,
        // so they get a plain CSS fade-in on the new view instead.
        await go();
        if (!reducedMotion) {
          const root = document.documentElement;
          root.dataset[demoRootData.showcaseEntering] = 'true';
          setTimeout(() => { delete root.dataset[demoRootData.showcaseEntering]; }, viewEnterMs);
        }
      }
    },
    target: (name) => waitForTarget(name),
    stepEvent: options.stepEvent,
    guide: (scope, action, target) => runGuideAction(document, scope, action, target),
    sleep: (ms, signal) => abortableSleep(ms / options.pace, signal),
    changed(state) {
      if (options.record && state.done) document.documentElement.dataset[demoRootData.showcaseDone] = 'true';
      options.changed(state);
    },
  }, { loop: !options.record, autoplay: !reducedMotion });
  const dispose = director.dispose;
  return Object.assign(director, { reducedMotion, dispose() { dispose(); stage.dispose(); } });
}
