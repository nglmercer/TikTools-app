import { describe, expect, it } from 'vitest';
import { showcaseSteps, type ShowcaseStep } from '../../../src/web/demo/showcase/script.ts';
import { parseDemoOptions } from '../../../src/web/demo/scenarios.ts';
import { isDemoFrameState, parseFrameCommand } from '../../../src/web/demo/frame-messages.ts';
import { createShowcaseHarness } from './support/showcase-harness.ts';
import { commandMessage, stateMessage } from './support/messages.ts';
import { FIXTURE_ID } from '../../shared/index.ts';

/** The stage log line for a step's spotlight, derived from the script so renamed ids stay in sync. */
const focusLog = (id: ShowcaseStep['id']) => {
  const step = showcaseSteps.find((item) => item.id === id)!;
  return `focus:${step.id}:${step.target}`;
};

describe('showcase script', () => {
  it('visits the five real views through registered routes and guide targets', () => {
    expect(showcaseSteps.map((step) => step.id)).toEqual(['feed', 'inspector', 'automation', 'overlays', 'points']);
    expect(showcaseSteps.map((step) => step.route)).toEqual(['/feed', '/feed', '/automations', '/overlays', '/points']);
    for (const step of showcaseSteps) expect(step.target).toMatch(/^[a-z-]+$/);
  });
});

describe('showcase director', () => {
  it('plays the first step: navigate, spotlight, three events, then dwell', async () => {
    const h = createShowcaseHarness();
    h.director.start();
    await h.flush();
    expect(h.log.slice(0, 3)).toEqual(['nav:/feed', focusLog('feed'), 'event']);
    for (let n = 0; n < 6; n += 1) await h.tick();
    expect(h.log.filter((entry) => entry === 'event')).toHaveLength(6);
    expect(h.sleeps.at(-1)?.ms).toBe(showcaseSteps[0]!.dwell);
    h.director.dispose();
  });

  it('moves the cursor before running a guide action and spotlights the result afterwards', async () => {
    const h = createShowcaseHarness();
    h.director.start();
    await h.flush();
    h.director.goto(1);
    await h.flush();
    const start = h.log.indexOf('point:feed-inspector-tab-user');
    expect(h.log.slice(start, start + 4)).toEqual(['point:feed-inspector-tab-user', 'click', 'guide:feed.feed.showViewer', focusLog('inspector')]);
    h.director.dispose();
  });

  it('spotlights a target that only exists after the guide action has run', async () => {
    const h = createShowcaseHarness({ late: ['feed-inspector-profile', 'automation-editor'] });
    h.director.start();
    await h.flush();
    h.director.goto(1);
    await h.flush();
    expect(h.log).toContain('guide:feed.feed.showViewer');
    expect(h.log.at(-1)).toBe(focusLog('inspector'));
    h.director.dispose();
  });

  it('advances through every step and loops back to the first', async () => {
    const h = createShowcaseHarness();
    h.director.start();
    const visited = new Set<number>();
    for (let n = 0; n < 40 && visited.size < 5; n += 1) {
      visited.add(h.director.state.step);
      await h.tick();
    }
    expect([...visited].sort()).toEqual([0, 1, 2, 3, 4]);
    for (let n = 0; n < 12; n += 1) await h.tick();
    expect(h.states.some((state, i) => i > 0 && state.step === 0 && h.states[i - 1]!.step === 4)).toBe(true);
    h.director.dispose();
  });

  it('stops and reports done after the last step when not looping (recording)', async () => {
    const h = createShowcaseHarness({ loop: false });
    h.director.start();
    for (let n = 0; n < 60 && !h.director.state.done; n += 1) await h.tick();
    expect(h.director.state).toMatchObject({ done: true, paused: true, step: 4 });
    expect(h.log.at(-1)).toBe('clear');
    h.director.dispose();
  });

  it('pauses by aborting the pending sleep and resumes the same step', async () => {
    const h = createShowcaseHarness();
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
    const h = createShowcaseHarness();
    h.director.start();
    await h.flush();
    h.director.goto(3);
    await h.flush();
    expect(h.director.state.step).toBe(3);
    expect(h.log).toContain('nav:/overlays');
    h.director.pause();
    h.director.goto(2);
    await h.flush();
    expect(h.log.at(-1)).toBe(focusLog('automation'));
    for (let n = 0; n < 4; n += 1) await h.tick();
    expect(h.director.state).toMatchObject({ step: 2, paused: true });
    h.director.dispose();
  });

  it('ignores invalid jumps', async () => {
    const h = createShowcaseHarness();
    h.director.start();
    await h.flush();
    for (const bad of [-1, 5, 1.5, Number.NaN]) h.director.goto(bad);
    expect(h.director.state.step).toBe(0);
    h.director.dispose();
  });

  it('skips a view whose target never renders instead of stalling', async () => {
    const h = createShowcaseHarness({ missing: ['feed-stage'] });
    h.director.start();
    await h.flush();
    expect(h.director.state.step).toBeGreaterThanOrEqual(1);
    expect(h.log).not.toContain('event');
    h.director.dispose();
  });

  it('releases the spotlight only when the next step is on another view', async () => {
    const h = createShowcaseHarness();
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
    const h = createShowcaseHarness({ autoplay: false });
    h.director.start();
    await h.flush();
    for (let n = 0; n < 10; n += 1) await h.tick();
    expect(h.director.state).toMatchObject({ step: 0, paused: true });
    h.director.dispose();
  });

  it('is inert after dispose', async () => {
    const h = createShowcaseHarness();
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
    for (const search of [
      '?showcase=2',
      '?showcase=1&embed=feed',
      '?showcase=1&preview=1',
      '?showcase=1&scenario=points',
      '?record=1',
      '?showcase=1&record=yes',
    ]) expect(() => parseDemoOptions(search), search).toThrow();
  });

  it('accepts a bounded showcase pace and rejects it everywhere else', () => {
    expect(parseDemoOptions('?showcase=1').pace).toBe(1);
    expect(parseDemoOptions('?showcase=1&pace=4').pace).toBe(4);
    for (const search of [
      '?pace=2',
      '?showcase=1&pace=0',
      '?showcase=1&pace=9',
      '?showcase=1&pace=1.5',
      '?showcase=1&pace=fast',
      '?showcase=1&pace=',
    ]) expect(() => parseDemoOptions(search), search).toThrow();
  });

  it('accepts showcase commands with bounded indices and validates the reported state', () => {
    expect(parseFrameCommand(commandMessage('showcase-play'), FIXTURE_ID.frame1)).toEqual({ action: 'showcase-play' });
    expect(parseFrameCommand(commandMessage('showcase-pause'), FIXTURE_ID.frame1)).toEqual({ action: 'showcase-pause' });
    expect(parseFrameCommand(commandMessage('showcase-goto', { value: 3 }), FIXTURE_ID.frame1)).toEqual({ action: 'showcase-goto', value: 3 });
    for (const value of [-1, 12, 1.5, '3', null]) expect(parseFrameCommand(commandMessage('showcase-goto', { value }), FIXTURE_ID.frame1)).toBeNull();
    const state = stateMessage({ paused: false, events: 3 });
    expect(isDemoFrameState({ ...state, showcase: { step: 2, count: 5, paused: false, done: false } })).toBe(true);
    expect(isDemoFrameState(state)).toBe(true);
    for (const showcase of [
      { step: 5, count: 5, paused: false, done: false },
      { step: 0, count: 99, paused: false, done: false },
      { step: 0, count: 5, paused: 'no', done: false },
      'x',
    ]) expect(isDemoFrameState({ ...state, showcase })).toBe(false);
  });
});
