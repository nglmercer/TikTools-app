import { createShowcaseDirector, type ShowcaseEnv, type ShowcaseState } from '../../../../src/web/demo/showcase/director.ts';
import { showcaseSteps, type ShowcaseStep } from '../../../../src/web/demo/showcase/script.ts';

/**
 * A hand-driven clock: every sleep waits until the test releases it.
 * `log` records each stage call as a short string, e.g. `focus:<step id>:<target>`.
 */
export function createShowcaseHarness(
  options: {
    missing?: string[];
    late?: string[];
    loop?: boolean;
    autoplay?: boolean
  } = {},
) {
  let revealed = false;
  const log: string[] = [];
  const states: ShowcaseState[] = [];
  const sleeps: Array<{
    ms: number;
    release(): void;
    signal: AbortSignal
  }> = [];
  const element = (name: string) => ({ name }) as unknown as HTMLElement;
  const env: ShowcaseEnv = {
    stage: {
      focus: async (el, step) => { log.push(`focus:${step.id}:${(el as unknown as { name: string }).name}`); },
      pointAt: async (el) => { log.push(`point:${(el as unknown as { name: string }).name}`); },
      click: async () => { log.push('click'); },
      release: () => { log.push('release'); },
      clear: () => { log.push('clear'); },
    },
    navigate: async (route) => { log.push(`nav:${route}`); },
    target: async (name) => (options.missing?.includes(name) || (!revealed && options.late?.includes(name)) ? null : element(name)),
    stepEvent: async () => { log.push('event'); },
    guide: async (scope, action) => {
      log.push(`guide:${scope}.${action}`);
      revealed = true;
      return true;
    },
    sleep: (ms, signal) => new Promise<void>((resolve, reject) => {
      const onAbort = () => reject(new DOMException('interrupted', 'AbortError'));
      if (signal.aborted) {
        onAbort();
        return;
      }
      signal.addEventListener('abort', onAbort, { once: true });
      sleeps.push({
        ms,
        signal,
        release: () => {
          signal.removeEventListener('abort', onAbort);
          resolve();
        },
      });
    }),
    changed: (state) => { states.push(state); },
  };
  const steps: readonly ShowcaseStep[] = showcaseSteps;
  const director = createShowcaseDirector(steps, env, { loop: options.loop ?? true, autoplay: options.autoplay ?? true });
  /** Release the oldest pending sleep and let the director run to its next one. */
  const tick = async () => {
    sleeps.shift()?.release();
    await new Promise((resolve) => setTimeout(resolve, 0));
  };
  const flush = () => new Promise((resolve) => setTimeout(resolve, 0));
  return { director, log, states, sleeps, tick, flush };
}
