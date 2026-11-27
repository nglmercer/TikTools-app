import { describe, expect, it } from 'vitest';
import { createShowcaseDirector, type ShowcaseEnv, type ShowcaseState } from '../../../src/web/demo/showcase/director.ts';
import { showcaseSteps, type ShowcaseStep } from '../../../src/web/demo/showcase/script.ts';
import { parseDemoOptions } from '../../../src/web/demo/scenarios.ts';
import { isDemoFrameState, parseFrameCommand } from '../../../src/web/demo/frame-messages.ts';

/** A hand-driven clock: every sleep waits until the test releases it. */
function harness(options: { missing?: string[]; late?: string[]; loop?: boolean; autoplay?: boolean } = {}) {
  let revealed = false;
  const log: string[] = [];
  const states: ShowcaseState[] = [];
  const sleeps: Array<{ ms: number; release(): void; signal: AbortSignal }> = [];
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
    guide: async (scope, action) => { log.push(`guide:${scope}.${action}`); revealed = true; return true; },
    sleep: (ms, signal) => new Promise<void>((resolve, reject) => {
      const onAbort = () => reject(new DOMException('interrupted', 'AbortError'));
      if (signal.aborted) { onAbort(); return; }
      signal.addEventListener('abort', onAbort, { once: true });
      sleeps.push({ ms, signal, release: () => { signal.removeEventListener('abort', onAbort); resolve(); } });
    }),
    changed: (state) => { states.push(state); },
  };
  const steps: readonly ShowcaseStep[] = showcaseSteps;
  const director = createShowcaseDirector(steps, env, { loop: options.loop ?? true, autoplay: options.autoplay ?? true });
  /** Release the oldest pending sleep and let the director run to its next one. */
  const tick = async () => { sleeps.shift()?.release(); await new Promise((resolve) => setTimeout(resolve, 0)); };
  const flush = () => new Promise((resolve) => setTimeout(resolve, 0));
  return { director, log, states, sleeps, tick, flush };
}

describe('showcase script', () => {
  it('visits the five real views through registered routes and guide targets', () => {
    expect(showcaseSteps.map((step) => step.id)).toEqual(['feed', 'inspector', 'automations', 'overlays', 'points']);
    expect(showcaseSteps.map((step) => step.route)).toEqual(['/feed', '/feed', '/automations', '/overlays', '/points']);
    for (const step of showcaseSteps) expect(step.target).toMatch(/^[a-z-]+$/);
  });
});

describe('showcase director', () => {
  it('plays the first step: navigate, spotlight, three events, then dwell', async () => {
    const h = harness();
    h.director.start();
    await h.flush();
    expect(h.log.slice(0, 3)).toEqual(['nav:/feed', 'focus:feed:feed-stage', 'event']);
    for (let n = 0; n < 6; n += 1) await h.tick();
    expect(h.log.filter((entry) => entry === 'event')).toHaveLength(6);
    expect(h.sleeps.at(-1)?.ms).toBe(showcaseSteps[0]!.dwell);
    h.director.dispose();
  });

  it('moves the cursor before running a guide action and spotlights the result afterwards', async () => {
    const h = harness();
    h.director.start();
    await h.flush();
    h.director.goto(1);
    await h.flush();
    const start = h.log.indexOf('point:feed-inspector-tab-user');
    expect(h.log.slice(start, start + 4)).toEqual(['point:feed-inspector-tab-user', 'click', 'guide:feed.feed.showViewer', 'focus:inspector:feed-inspector-profile']);
    h.director.dispose();
  });

  it('spotlights a target that only exists after the guide action has run', async () => {
    const h = harness({ late: ['feed-inspector-profile', 'automation-editor'] });
    h.director.start();
    await h.flush();
    h.director.goto(1);
    await h.flush();
    expect(h.log).toContain('guide:feed.feed.showViewer');
    expect(h.log.at(-1)).toBe('focus:inspector:feed-inspector-profile');
    h.director.dispose();
  });

  it('advances through every step and loops back to the first', async () => {
    const h = harness();
    h.director.start();
    const visited = new Set<number>();
    for (let n = 0; n < 40 && visited.size < 5; n += 1) { visited.add(h.director.state.step); await h.tick(); }
    expect([...visited].sort()).toEqual([0, 1, 2, 3, 4]);
    for (let n = 0; n < 12; n += 1) await h.tick();
    expect(h.states.some((state, i) => i > 0 && state.step === 0 && h.states[i - 1]!.step === 4)).toBe(true);
    h.director.dispose();
  });

  it('stops and reports done after the last step when not looping (recording)', async () => {
    const h = harness({ loop: false });
    h.director.start();
    for (let n = 0; n < 60 && !h.director.state.done; n += 1) await h.tick();
    expect(h.director.state).toMatchObject({ done: true, paused: true, step: 4 });
    expect(h.log.at(-1)).toBe('clear');
    h.director.dispose();
  });

  it('pauses by aborting the pending sleep and resumes the same step', async () => {
    const h = harness();
    h.director.start();
    await h.flush();
    const before = h.director.state.step;
    h.director.pause();
    await h.flush();
    expect(h.director.state.paused).toBe(true);
    expect(h.sleeps.at(-1)?.signal.aborted).toBe(true);
    const events = h.log.filter((entry) => entry === 'event').length;
    await h.flush();
    expect(h.log.filter((entry) => entry === 'event')).toHaveLength(events);
    h.director.play();
    await h.flush();
    expect(h.director.state).toMatchObject({ paused: false, step: before });
    h.director.dispose();
  });

  it('jumps to a step and keeps playing, or shows it once when paused', async () => {
    const h = harness();
    h.director.start();
    await h.flush();
    h.director.goto(3);
    await h.flush();
    expect(h.director.state.step).toBe(3);
    expect(h.log).toContain('nav:/overlays');
    h.director.pause();
    h.director.goto(2);
    await h.flush();
    expect(h.log.at(-1)).toBe('focus:automations:automation-editor');
    for (let n = 0; n < 4; n += 1) await h.tick();
    expect(h.director.state).toMatchObject({ step: 2, paused: true });
    h.director.dispose();
  });

  it('ignores invalid jumps', async () => {
    const h = harness();
    h.director.start();
    await h.flush();
    for (const bad of [-1, 5, 1.5, Number.NaN]) h.director.goto(bad);
    expect(h.director.state.step).toBe(0);
    h.director.dispose();
  });

  it('skips a view whose target never renders instead of stalling', async () => {
    const h = harness({ missing: ['feed-stage'] });
    h.director.start();
    await h.flush();
    expect(h.director.state.step).toBeGreaterThanOrEqual(1);
    expect(h.log).not.toContain('event');
    h.director.dispose();
  });

  it('releases the spotlight only when the next step is on another view', async () => {
    const h = harness();
    h.director.start();
    await h.flush();
    h.director.goto(1);
    await h.flush();
    expect(h.log).not.toContain('release');
    h.director.goto(2);
    await h.flush();
    expect(h.log.indexOf('release')).toBeLessThan(h.log.indexOf('nav:/automations'));
    h.director.dispose();
  });

  it('with autoplay off (reduced motion) shows only the first step and waits', async () => {
    const h = harness({ autoplay: false });
    h.director.start();
    await h.flush();
    for (let n = 0; n < 10; n += 1) await h.tick();
    expect(h.director.state).toMatchObject({ step: 0, paused: true });
    h.director.dispose();
  });

  it('is inert after dispose', async () => {
    const h = harness();
    h.director.start();
    await h.flush();
    h.director.dispose();
    const count = h.log.length;
    h.director.play();
    h.director.goto(2);
    await h.flush();
    expect(h.log.length).toBe(count);
  });
});

describe('showcase options and frame bridge', () => {
  it('defaults the showcase to the feed scenario and rejects mixing it with tours, panels or previews', () => {
    expect(parseDemoOptions('?showcase=1')).toMatchObject({ showcase: true, scenario: 'feed', route: '/feed', record: false });
    expect(parseDemoOptions('?showcase=1&record=1&framed=1')).toMatchObject({ showcase: true, record: true, framed: true });
    expect(parseDemoOptions('')).toMatchObject({ showcase: false, record: false });
    for (const search of ['?showcase=2', '?showcase=1&embed=feed', '?showcase=1&preview=1', '?showcase=1&scenario=points', '?record=1', '?showcase=1&record=yes']) expect(() => parseDemoOptions(search), search).toThrow();
  });

  it('accepts showcase commands with bounded indices and validates the reported state', () => {
    const command = { type: 'tiktools.demo.command', instance: 'frame-1' };
    expect(parseFrameCommand({ ...command, action: 'showcase-play' }, 'frame-1')).toEqual({ action: 'showcase-play' });
    expect(parseFrameCommand({ ...command, action: 'showcase-pause' }, 'frame-1')).toEqual({ action: 'showcase-pause' });
    expect(parseFrameCommand({ ...command, action: 'showcase-goto', value: 3 }, 'frame-1')).toEqual({ action: 'showcase-goto', value: 3 });
    for (const value of [-1, 12, 1.5, '3', null]) expect(parseFrameCommand({ ...command, action: 'showcase-goto', value }, 'frame-1')).toBeNull();
    const state = { type: 'tiktools.demo.state', instance: 'frame-1', ready: true, paused: false, busy: false, events: 3, status: 'paused', connectionCase: 'normal' };
    expect(isDemoFrameState({ ...state, showcase: { step: 2, count: 5, paused: false, done: false } })).toBe(true);
    expect(isDemoFrameState(state)).toBe(true);
    for (const showcase of [{ step: 5, count: 5, paused: false, done: false }, { step: 0, count: 99, paused: false, done: false }, { step: 0, count: 5, paused: 'no', done: false }, 'x']) expect(isDemoFrameState({ ...state, showcase })).toBe(false);
  });
});
