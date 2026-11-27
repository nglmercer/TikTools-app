import type { ShowcaseStep } from './script.ts';

export interface ShowcaseState { step: number; count: number; paused: boolean; done: boolean }

/** What the presentation layer must do; the director never touches the DOM itself. */
export interface ShowcaseStage {
  /** Spotlight `element` and show the tooltip of `step`. Resolves when the spotlight has settled. */
  focus(element: HTMLElement, step: ShowcaseStep): Promise<void>;
  /** Move the cursor over `element` and resolve on arrival. */
  pointAt(element: HTMLElement): Promise<void>;
  click(): Promise<void>;
  /** Fade the spotlight and tooltip out (the previous target is about to disappear). */
  release(): void;
  clear(): void;
}

export interface ShowcaseEnv {
  stage: ShowcaseStage;
  navigate(route: string): Promise<void>;
  /** Resolves with the visible element for a `data-guide` target, or null after a timeout. */
  target(name: string): Promise<HTMLElement | null>;
  stepEvent(): Promise<void>;
  guide(scope: string, action: string, target: string): Promise<boolean>;
  /** Rejects with an AbortError when `signal` aborts. */
  sleep(ms: number, signal: AbortSignal): Promise<void>;
  changed(state: ShowcaseState): void;
}

export interface ShowcaseOptions {
  /** Restart from the first step after the last one. */
  loop: boolean;
  /** Keep playing after a jump or a resume. A reduced-motion run shows one step and waits. */
  autoplay: boolean;
}

export interface ShowcaseDirector {
  readonly state: ShowcaseState;
  start(): void;
  play(): void;
  pause(): void;
  goto(index: number): void;
  dispose(): void;
}

const isAbort = (error: unknown) => error instanceof DOMException && error.name === 'AbortError';

export function createShowcaseDirector(steps: readonly ShowcaseStep[], env: ShowcaseEnv, options: ShowcaseOptions): ShowcaseDirector {
  let index = 0;
  let paused = true;
  let done = false;
  let disposed = false;
  let controller = new AbortController();
  let route: string | undefined;
  const snapshot = (): ShowcaseState => ({ step: index, count: steps.length, paused, done });
  const emit = () => { if (!disposed) env.changed(snapshot()); };
  const aborted = (signal: AbortSignal) => { if (signal.aborted) throw new DOMException('Showcase step interrupted.', 'AbortError'); };

  async function playStep(step: ShowcaseStep, signal: AbortSignal) {
    // A new view removes the old target, so drop its spotlight instead of leaving it on empty space.
    if (route !== undefined && route !== step.route) env.stage.release();
    route = step.route;
    await env.navigate(step.route);
    aborted(signal);
    const focus = (element: HTMLElement) => env.stage.focus(element, step);
    // A target that an operation reveals (the inspector profile, an open editor) cannot be awaited up front.
    if (step.focusFirst) {
      const target = await env.target(step.target);
      aborted(signal);
      // A view that does not render its target in time is skipped, never waited on.
      if (!target) return;
      await focus(target);
    }
    for (const op of step.ops) {
      aborted(signal);
      if (op.kind === 'events') {
        for (let n = 0; n < op.count; n += 1) { await env.stepEvent(); aborted(signal); await env.sleep(op.gap, signal); }
      } else {
        if (op.cursor) {
          const pointer = await env.target(op.cursor);
          aborted(signal);
          if (pointer) { await env.stage.pointAt(pointer); aborted(signal); await env.stage.click(); aborted(signal); }
        }
        await env.guide(step.scope, op.action, step.target);
        aborted(signal);
      }
    }
    if (!step.focusFirst) {
      const settled = await env.target(step.target);
      aborted(signal);
      if (!settled) return;
      await focus(settled);
    }
    aborted(signal);
    await env.sleep(step.dwell, signal);
  }

  async function drive(continuous: boolean) {
    controller.abort();
    const mine = new AbortController();
    controller = mine;
    try {
      for (;;) {
        done = false;
        emit();
        await playStep(steps[index]!, mine.signal);
        if (!continuous) { paused = true; emit(); return; }
        if (index === steps.length - 1) {
          if (!options.loop) { paused = true; done = true; env.stage.clear(); emit(); return; }
          index = 0;
        } else index += 1;
      }
    } catch (error) {
      if (!isAbort(error)) { paused = true; emit(); throw error; }
    }
  }
  const run = (continuous: boolean) => { void drive(continuous).catch(() => undefined); };

  return {
    get state() { return snapshot(); },
    start() { if (disposed) return; paused = !options.autoplay; run(options.autoplay); },
    play() { if (disposed || !paused) return; paused = false; if (done) index = 0; run(true); },
    pause() { if (disposed || paused) return; paused = true; controller.abort(); emit(); },
    goto(next) {
      if (disposed || !Number.isInteger(next) || next < 0 || next >= steps.length) return;
      index = next;
      run(!paused);
    },
    dispose() { disposed = true; controller.abort(); env.stage.clear(); },
  };
}
