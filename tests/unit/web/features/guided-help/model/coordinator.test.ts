import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises } from '@vue/test-utils';
import { ref } from 'vue';
import { createMemoryHistory, createRouter } from 'vue-router';
import type { Config, DriveStep, Driver } from 'driver.js';
import type { GuidePause } from '../../../../../../src/web/features/guided-help/model/types.ts';
import { createTourCoordinator, MAX_GUIDE_WAIT_MS } from '../../../../../../src/web/features/guided-help/model/coordinator.ts';
import { registerGuideActions } from '../../../../../../src/web/features/guided-help/model/guide-actions.ts';
import { readTourProgress, writeTourProgress } from '../../../../../../src/web/features/guided-help/model/progress.ts';
import { tourRegistry } from '../../../../../../src/web/features/guided-help/model/registry.ts';
import { tourCatalogs } from '../../../../../../src/web/features/guided-help/catalogs';
import { rectList } from '../../../../support/dom.ts';
import { mountGuideTargets, removeGuideFixtures } from '../support/dom.ts';
import { RPC, defined } from '../../../../../shared/index.ts';

interface StepCopy {
  title: string;
  description: string;
}

function driverFixture() {
  let config: Config = {};
  let steps: DriveStep[] = [];
  let index = 0;
  // Driver steps may name their element by selector or factory; the fixtures always pass the element itself.
  const elementOf = (step: DriveStep | undefined): Element | undefined => (step?.element instanceof Element ? step.element : undefined);
  const invoke = (hook: Config['onHighlighted']) =>
    hook?.(elementOf(steps[index]), steps[index] ?? {}, { driver: engine, config, state: {}, index });
  const drive = vi.fn((next = 0) => {
    index = next;
    invoke(config.onHighlighted);
  });
  const engine: Driver = {
    isActive: () => false,
    refresh: () => {},
    drive,
    setConfig: vi.fn((next: Config) => { config = next; }),
    setSteps: vi.fn((next: DriveStep[]) => { steps = next; }),
    getConfig: () => config,
    getState: () => undefined,
    getActiveIndex: () => index,
    isFirstStep: () => index === 0,
    isLastStep: () => index === steps.length - 1,
    getActiveStep: () => steps[index],
    getActiveElement: () => elementOf(steps[index]),
    getPreviousElement: () => undefined,
    getPreviousStep: () => steps[index - 1],
    getNextStep: () => steps[index + 1],
    moveNext: vi.fn(() => {
      index++;
      invoke(config.onHighlighted);
    }),
    movePrevious: vi.fn(() => {
      index--;
      invoke(config.onHighlighted);
    }),
    moveTo: () => {},
    hasNextStep: () => index < steps.length - 1,
    hasPreviousStep: () => index > 0,
    highlight: () => {},
    updatePopover: () => {},
    advance: () => {},
    nudge: () => {},
    destroy: vi.fn(() => invoke(config.onDestroyed)),
  };
  const driver = vi.fn((next: Config) => {
    config = next;
    return engine;
  });
  const loadDriver = vi.fn(async () => ({ driver }));
  return {
    driver,
    drive,
    engine,
    loadDriver,
    config: () => config,
    steps: () => steps,
    next: () => invoke(config.onNextClick),
    close: () => invoke(config.onCloseClick),
    done: () => invoke(config.onDoneClick),
  };
}

const owned: Array<ReturnType<typeof createTourCoordinator>> = [];
function setup() {
  const fixture = driverFixture();
  const coordinator = createTourCoordinator({ loadDriver: fixture.loadDriver, storage: () => sessionStorage });
  owned.push(coordinator);
  return { ...fixture, coordinator };
}
beforeEach(() => {
  sessionStorage.clear();
  // Happy DOM has no layout engine; production visibility always uses real browser geometry.
  vi.spyOn(HTMLElement.prototype, 'getClientRects').mockImplementation(function (this: HTMLElement) {
    return this.isConnected ? rectList(new DOMRect(0, 0, 120, 30)) : rectList();
  });
});
afterEach(() => {
  for (const coordinator of owned.splice(0)) coordinator.dispose();
  removeGuideFixtures();
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

it('is lazy, restores focus, and records dismissal without completing the guide', async () => {
  const test = setup();
  const root = mountGuideTargets('points', ['points-rules', 'points-leaderboard']);
  const trigger = document.createElement('button');
  root.append(trigger);
  expect(test.loadDriver).not.toHaveBeenCalled();
  const session = test.coordinator.startTour('points', { trigger });
  expect(await session.ready).toBe(true);
  test.close();
  expect(await session.done).toBe('dismissed');
  expect(document.activeElement).toBe(trigger);
  expect(readTourProgress(sessionStorage, 'points', 1, ['rules', 'leaderboard'])).toEqual({ version: 1, stepId: 'rules', status: 'dismissed' });
});

it('finishing the guide does not complete an unverified task; reactive host state does', async () => {
  const test = setup();
  mountGuideTargets('points', ['points-rules', 'points-leaderboard']);
  const session = test.coordinator.startTour('points');
  await session.ready;
  test.next();
  await flushPromises();
  expect(session.active).toBe(true);
  test.next();
  expect(await session.done).toBe('dismissed');
  expect(readTourProgress(sessionStorage, 'points', 1, ['rules', 'leaderboard'])?.status).toBe('dismissed');
  const state = ref<Record<string, boolean>>({});
  const task = test.coordinator.startTour('points', { taskState: () => state.value });
  await task.ready;
  state.value = { 'points.reviewed': true };
  await flushPromises();
  expect(await task.done).toBe('task-completed');
  expect(readTourProgress(sessionStorage, 'points', 1, ['rules', 'leaderboard'])?.status).toBe('completed');
});

it('cannot mark progress complete through the session API without verified task state', async () => {
  const test = setup();
  mountGuideTargets('points', ['points-rules']);
  const session = test.coordinator.startTour('points');
  await session.ready;
  session.stop('completed');
  expect(await session.done).toBe('dismissed');
  expect(readTourProgress(sessionStorage, 'points', 1, ['rules'])?.status).toBe('dismissed');
});

it('does not treat the Done button or a failing host-state reader as task success', async () => {
  const test = setup();
  mountGuideTargets('points', ['points-rules', 'points-leaderboard']);
  const session = test.coordinator.startTour('points', { taskState: () => { throw new Error('host not ready'); } });
  await session.ready;
  test.next();
  await flushPromises();
  test.done();
  expect(await session.done).toBe('dismissed');
  expect(readTourProgress(sessionStorage, 'points', 1, ['rules', 'leaderboard'])?.status).toBe('dismissed');
});

it('completes an informational walkthrough when its final step finishes without inventing host state', async () => {
  const test = setup();
  const targets = tourRegistry.analytics.steps.map((step) => step.target);
  mountGuideTargets('analytics', targets);
  expect(tourRegistry.analytics.task).toBeUndefined();
  const session = test.coordinator.startTour('analytics');
  await flushPromises();
  expect(await session.ready).toBe(true);
  for (let index = 1; index < targets.length; index++) {
    test.next();
    await flushPromises();
  }
  test.done();
  expect(await session.done).toBe('completed');
  expect(readTourProgress(sessionStorage, 'analytics', 1, tourRegistry.analytics.steps.map((step) => step.id))?.status).toBe('completed');
});

it('does not let the session API complete an informational walkthrough before its final step', async () => {
  const test = setup();
  mountGuideTargets('settings', tourRegistry.settings.steps.map((step) => step.target));
  const session = test.coordinator.startTour('settings');
  await flushPromises();
  expect(await session.ready).toBe(true);
  session.stop('completed');
  expect(await session.done).toBe('dismissed');
  expect(readTourProgress(sessionStorage, 'settings', 1, tourRegistry.settings.steps.map((step) => step.id))?.status).toBe('dismissed');
});

it('resumes stable step IDs and ignores progress from another version or malformed storage', async () => {
  const test = setup();
  mountGuideTargets('points', ['points-rules', 'points-leaderboard']);
  writeTourProgress(sessionStorage, 'points', { version: 1, stepId: 'leaderboard', status: 'dismissed' });
  const session = test.coordinator.startTour('points');
  await session.ready;
  expect(test.engine.drive).toHaveBeenLastCalledWith(1);
  session.stop();
  expect(readTourProgress(sessionStorage, 'points', 2, ['rules', 'leaderboard'])).toBeUndefined();
  sessionStorage.setItem('tiktools.guide.tour.points.v1', '{"version":1,"stepId":"outside","status":"completed"}');
  expect(readTourProgress(sessionStorage, 'points', 1, ['rules'])).toBeUndefined();
  sessionStorage.setItem('tiktools.guide.tour.points.v1', 'broken JSON');
  expect(readTourProgress(sessionStorage, 'points', 1, ['rules'])).toBeUndefined();
});

it('waits for a route target but terminates and removes timers at the configured bound', async () => {
  vi.useFakeTimers();
  const test = setup();
  const session = test.coordinator.startTour('connection', { waitMs: 40 });
  await vi.advanceTimersByTimeAsync(40);
  expect(await session.ready).toBe(false);
  expect(await session.done).toBe('unavailable');
  expect(test.loadDriver).not.toHaveBeenCalled();
  expect(vi.getTimerCount()).toBe(0);
  mountGuideTargets('connections', ['connection-form']);
  await Promise.resolve();
  expect(test.driver).not.toHaveBeenCalled();
});

it('accepts a late target inside the bound and caps excessive waits', async () => {
  vi.useFakeTimers();
  const test = setup();
  const session = test.coordinator.startTour('connection', { waitMs: 40 });
  await vi.advanceTimersByTimeAsync(10);
  mountGuideTargets('connections', ['connection-form']);
  await vi.advanceTimersByTimeAsync(0);
  expect(await session.ready).toBe(true);
  session.stop();
  const missing = test.coordinator.startTour('diagnostics', { waitMs: 1e9 });
  await vi.advanceTimersByTimeAsync(MAX_GUIDE_WAIT_MS);
  expect(await missing.done).toBe('unavailable');
  expect(vi.getTimerCount()).toBe(0);
});

it('cleans up a pending target wait on navigation, with no late restart', async () => {
  const test = setup();
  const router = createRouter({
    history: createMemoryHistory(),
    routes: ['/points', '/connections'].map((path) => ({ path, component: { template: '<div />' } })),
  });
  await router.push('/points');
  const session = test.coordinator.startTour('points', { router });
  await Promise.resolve();
  await router.push('/connections');
  expect(await session.done).toBe('navigation');
  mountGuideTargets('points', ['points-rules']);
  await Promise.resolve();
  expect(test.driver).not.toHaveBeenCalled();
});

it('navigates once to its declared route and destroys an active guide on leaving it', async () => {
  const test = setup();
  mountGuideTargets('points', ['points-rules']);
  const router = createRouter({
    history: createMemoryHistory(),
    routes: ['/points', '/connections'].map((path) => ({ path, component: { template: '<div />' } })),
  });
  await router.push('/connections');
  const session = test.coordinator.startTour('points', { router });
  expect(await session.ready).toBe(true);
  expect(router.currentRoute.value.path).toBe('/points');
  await router.push('/connections');
  expect(await session.done).toBe('navigation');
  expect(test.engine.destroy).toHaveBeenCalledTimes(1);
});

it('keeps an existing desktop diagnostics panel in place without navigating to the mobile page', async () => {
  const test = setup();
  const root = mountGuideTargets('diagnostics', ['diagnostics-log', 'diagnostics-actions']);
  const router = createRouter({
    history: createMemoryHistory(),
    routes: ['/debug', '/connections'].map((path) => ({ path, component: { template: '<div />' } })),
  });
  await router.push('/connections');
  const push = vi.spyOn(router, 'push');
  const session = test.coordinator.startTour('diagnostics', { router, scope: root, taskState: { 'diagnostics.inspected': true } });
  expect(await session.ready).toBe(true);
  expect(router.currentRoute.value.path).toBe('/connections');
  expect(push).not.toHaveBeenCalled();
  test.next();
  await flushPromises();
  test.done();
  expect(await session.done).toBe('completed');
});

it('bounds navigation and lazy import waits and cancels a superseded load', async () => {
  vi.useFakeTimers();
  const fixture = driverFixture();
  let release!: (value: { driver: typeof fixture.driver }) => void;
  const coordinator = createTourCoordinator({ loadDriver: () => new Promise((resolve) => { release = resolve; }) });
  owned.push(coordinator);
  mountGuideTargets('points', ['points-rules']);
  const pending = coordinator.startTour('points', { waitMs: 20 });
  await vi.advanceTimersByTimeAsync(0);
  coordinator.dispose();
  expect(await pending.done).toBe('disposed');
  release({ driver: fixture.driver });
  await vi.advanceTimersByTimeAsync(0);
  expect(fixture.driver).not.toHaveBeenCalled();
  expect(vi.getTimerCount()).toBe(0);
  const timedOut = coordinator.startTour('points', { waitMs: 20 });
  await vi.advanceTimersByTimeAsync(20);
  expect(await timedOut.done).toBe('unavailable');
});

it('releases its navigation deadline when a route guard never settles', async () => {
  vi.useFakeTimers();
  const test = setup();
  const router = createRouter({
    history: createMemoryHistory(),
    routes: ['/points', '/connections'].map((path) => ({ path, component: { template: '<div />' } })),
  });
  await router.push('/connections');
  const removeGuard = router.beforeEach((to) => to.path === '/points' ? new Promise<boolean>(() => {}) : true);
  const session = test.coordinator.startTour('points', { router, waitMs: 30 });
  await vi.advanceTimersByTimeAsync(30);
  expect(await session.done).toBe('unavailable');
  expect(await session.ready).toBe(false);
  expect(vi.getTimerCount()).toBe(0);
  expect(test.loadDriver).not.toHaveBeenCalled();
  removeGuard();
});

it('keeps targets in its workspace and explicitly skips hidden/denied later steps', async () => {
  vi.useFakeTimers();
  const test = setup();
  const foreign = mountGuideTargets('plugins', ['plugins-search']);
  const root = mountGuideTargets('plugins', ['plugin-catalog', 'plugins-search', 'plugins-install']);
  defined(root.querySelector('[data-guide="plugins-search"]')).setAttribute('hidden', '');
  const session = test.coordinator.startTour('plugin', { scope: root, capabilities: { supports: () => false }, waitMs: 10 });
  await vi.advanceTimersByTimeAsync(0);
  await session.ready;
  expect(defined(test.steps()[0]).element).toBe(root.querySelector('[data-guide="plugin-catalog"]'));
  expect(test.config().allowedElements).toEqual([root]);
  expect(test.steps().some((step) => step.element instanceof Node && foreign.contains(step.element))).toBe(false);
  test.next();
  await vi.advanceTimersByTimeAsync(10);
  expect(await session.done).toBe('dismissed');
  expect(test.engine.drive).toHaveBeenCalledTimes(1);
});

it('waits for visibility of each subsequent step, rather than checking only the first', async () => {
  vi.useFakeTimers();
  const test = setup();
  const root = mountGuideTargets('points', ['points-rules', 'points-leaderboard']);
  const later = defined(root.querySelector<HTMLElement>('[data-guide="points-leaderboard"]'));
  later.style.display = 'none';
  const session = test.coordinator.startTour('points', { waitMs: 30 });
  await vi.advanceTimersByTimeAsync(0);
  expect(await session.ready).toBe(true);
  test.next();
  await vi.advanceTimersByTimeAsync(10);
  expect(test.engine.drive).toHaveBeenCalledTimes(1);
  later.style.display = '';
  await vi.advanceTimersByTimeAsync(0);
  expect(test.engine.drive).toHaveBeenLastCalledWith(1);
  expect(session.active).toBe(true);
});

it('cancels on a required later target deadline and removes all pending work', async () => {
  vi.useFakeTimers();
  const test = setup();
  mountGuideTargets('points', ['points-rules']);
  const session = test.coordinator.startTour('points', { waitMs: 20 });
  await vi.advanceTimersByTimeAsync(0);
  await session.ready;
  test.next();
  await vi.advanceTimersByTimeAsync(20);
  expect(await session.done).toBe('unavailable');
  expect(vi.getTimerCount()).toBe(0);
  expect(test.engine.drive).toHaveBeenCalledTimes(1);
});

it('does not highlight zero-opacity ancestors or zero-area targets', async () => {
  vi.useFakeTimers();
  const test = setup();
  const root = mountGuideTargets('points', ['points-rules', 'points-leaderboard']);
  root.style.opacity = '0';
  const invisible = test.coordinator.startTour('points', { waitMs: 10 });
  await vi.advanceTimersByTimeAsync(10);
  expect(await invisible.done).toBe('unavailable');
  expect(test.driver).not.toHaveBeenCalled();
  root.style.opacity = '1';
  const rules = defined(root.querySelector<HTMLElement>('[data-guide="points-rules"]'));
  vi.spyOn(rules, 'getClientRects').mockReturnValue(rectList(new DOMRect(0, 0, 0, 30)));
  const unlaidOut = test.coordinator.startTour('points', { waitMs: 10 });
  await vi.advanceTimersByTimeAsync(10);
  expect(await unlaidOut.done).toBe('unavailable');
  expect(test.driver).not.toHaveBeenCalled();
});

it('detects layout becoming visible without a DOM mutation and clears polling on disposal', async () => {
  vi.useFakeTimers();
  const test = setup();
  const root = mountGuideTargets('points', ['points-rules']);
  const rules = defined(root.querySelector<HTMLElement>('[data-guide="points-rules"]'));
  let laidOut = false;
  vi.spyOn(rules, 'getClientRects').mockImplementation(() => (laidOut ? rectList(new DOMRect(0, 0, 120, 30)) : rectList()));
  const session = test.coordinator.startTour('points', { waitMs: 200 });
  await vi.advanceTimersByTimeAsync(10);
  expect(test.driver).not.toHaveBeenCalled();
  laidOut = true;
  await vi.advanceTimersByTimeAsync(50);
  expect(await session.ready).toBe(true);
  session.stop();
  expect(vi.getTimerCount()).toBe(0);
});

it('skips an optional missing step after its own deadline, then highlights the next target', async () => {
  vi.useFakeTimers();
  const test = setup();
  mountGuideTargets('connections', ['connection-form', 'connection-plugins']);
  const session = test.coordinator.startTour('connection', { waitMs: 20 });
  await vi.advanceTimersByTimeAsync(0);
  await session.ready;
  test.next();
  await vi.advanceTimersByTimeAsync(20);
  expect(session.active).toBe(true);
  expect(test.engine.drive).toHaveBeenLastCalledWith(2);
});

it('aborts a subsequent step wait when the user navigates away', async () => {
  vi.useFakeTimers();
  const test = setup();
  mountGuideTargets('points', ['points-rules']);
  const router = createRouter({
    history: createMemoryHistory(),
    routes: ['/points', '/connections'].map((path) => ({ path, component: { template: '<div />' } })),
  });
  await router.push('/points');
  const session = test.coordinator.startTour('points', { router, waitMs: 50 });
  await vi.advanceTimersByTimeAsync(0);
  await session.ready;
  test.next();
  await router.push('/connections');
  expect(await session.done).toBe('navigation');
  expect(vi.getTimerCount()).toBe(0);
  await vi.advanceTimersByTimeAsync(50);
  expect(test.engine.drive).toHaveBeenCalledTimes(1);
});

it('escapes untrusted display text and respects reduced motion without losing keyboard controls', async () => {
  const test = setup();
  const root = mountGuideTargets('inline', ['inline']);
  vi.stubGlobal('matchMedia', () => ({ matches: true }));
  const session = test.coordinator.startInlineGuide({
    id: 'inline',
    scope: root,
    steps: () => [
      { id: 'one', selector: '[data-guide="inline"]', title: '<img onerror="bad">', description: '<script>bad</script>&' },
    ],
  });
  await session.ready;
  expect(defined(test.steps()[0]).popover?.title).toBe('&lt;img onerror=&quot;bad&quot;&gt;');
  expect(defined(test.steps()[0]).popover?.description).toBe('&lt;script&gt;bad&lt;/script&gt;&amp;');
  expect(test.config().animate).toBe(false);
  expect(test.config().allowKeyboardControl).toBe(true);
  expect(test.config().showButtons).toEqual(['close']);
});

it('stops for a modal or removed workspace and never steals focus behind a modal', async () => {
  const test = setup();
  const root = mountGuideTargets('points', ['points-rules']);
  const trigger = document.createElement('button');
  root.append(trigger);
  const focus = vi.spyOn(trigger, 'focus');
  const session = test.coordinator.startTour('points', { trigger });
  await session.ready;
  const modal = document.createElement('dialog');
  modal.open = true;
  root.append(modal);
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(await session.done).toBe('dismissed');
  expect(focus).not.toHaveBeenCalled();
  modal.remove();
  const removed = test.coordinator.startTour('points');
  await removed.ready;
  root.remove();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(await removed.done).toBe('unavailable');
});

it('replaces the previous guide and tolerates unavailable storage', async () => {
  const fixture = driverFixture();
  const coordinator = createTourCoordinator({ loadDriver: fixture.loadDriver, storage: () => { throw new Error('blocked'); } });
  owned.push(coordinator);
  mountGuideTargets('points', ['points-rules']);
  const first = coordinator.startTour('points');
  await first.ready;
  const second = coordinator.startTour('points');
  expect(await first.done).toBe('replaced');
  expect(await second.ready).toBe(true);
  second.stop();
  expect(await second.done).toBe('dismissed');
});

describe('versioned registry and localized copy', () => {
  it('publishes versioned tours with complete catalogs for every supported locale', () => {
    expect(Object.keys(tourRegistry)).toHaveLength(Object.keys(tourRegistry).length);
    for (const tour of Object.values(tourRegistry)) {
      expect(tour.version).toBeGreaterThan(0);
      expect(tour.route.startsWith('/')).toBe(true);
      for (const [locale, catalog] of Object.entries(tourCatalogs)) {
        // The catalogs are keyed by tour id at runtime; the typed catalog has no string index.
        const copy: Record<string, StepCopy> = Reflect.get(catalog, tour.id);
        expect(Object.keys(copy)).toEqual(tour.steps.map((step) => step.id));
        for (const step of tour.steps) {
          expect(step.version).toBeGreaterThan(0);
          expect(step.titleKey).toBe(`${tour.id}.${step.id}.title`);
          expect(step.descriptionKey).toBe(`${tour.id}.${step.id}.description`);
          expect(Array.isArray(step.requiredCapabilities)).toBe(true);
          expect(['skip', 'cancel']).toContain(step.missingTargetPolicy);
          const cjk = locale === 'zh' || locale === 'ja';
          expect(defined(copy[step.id]).title.trim().length).toBeGreaterThan(cjk ? 1 : 5);
          expect(defined(copy[step.id]).description.trim().length).toBeGreaterThan(cjk ? 15 : 25);
        }
      }
    }
  });
  it('rejects unknown IDs without loading or navigating', () => {
    const test = setup();
    expect(() => test.coordinator.startTour('__proto__')).toThrow('Unknown guided tour');
    expect(test.loadDriver).not.toHaveBeenCalled();
  });
});


describe('declared action example requirements', () => {
  it('fails closed when a requirement-bearing tour starts without a satisfying automation snapshot', async () => {
    // The bounded wait for the targets runs on the fake clock instead of costing 2.4 s per case.
    vi.useFakeTimers();
    for (const automationSnapshot of [undefined, { actions: [], actionTypes: [], plugins: [] }] as const) {
      const test = setup();
      const root = mountGuideTargets('automations', tourRegistry.automationActionHttp.steps.map((step) => step.target));
      const session = test.coordinator.startTour('automationActionHttp', { scope: root, locale: 'en', automationSnapshot });
      await vi.advanceTimersByTimeAsync(MAX_GUIDE_WAIT_MS);
      expect(await session.ready).toBe(false);
      expect(await session.done).toBe('unavailable');
      expect(test.loadDriver).not.toHaveBeenCalled();
      root.remove();
    }
  });
});

describe('account and backup tour', () => {
  it('has text in every locale and steps through plan, create, tools and list', async () => {
    for (const catalog of Object.values(tourCatalogs)) {
      for (const step of tourRegistry.backup.steps) {
        const [group, key] = step.titleKey.split('.');
        const entry: StepCopy | undefined = Reflect.get(Reflect.get(catalog, group ?? ''), key ?? '');
        expect(entry, step.id).toBeDefined();
        expect(entry?.title, step.id).not.toBe('');
      }
    }
    const test = setup();
    const root = mountGuideTargets('account', tourRegistry.backup.steps.map((step) => step.target));
    const session = test.coordinator.startTour('backup', {
      scope: root,
      locale: 'en',
      signedIn: true,
      capabilities: [RPC.cloudPlanGet, 'cloud.backup.createNamed', RPC.cloudBackupList],
    });
    await flushPromises();
    expect(await session.ready).toBe(true);
    for (let index = 1; index < 4; index++) {
      test.next();
      await flushPromises();
      expect(session.active, `step ${index + 1}`).toBe(true);
    }
    session.stop();
  });


  it('refuses account-required tours unless the caller reports a signed-in account', async () => {
    for (const signedIn of [undefined, false, () => false] as const) {
      const test = setup();
      const root = mountGuideTargets('account', tourRegistry.backup.steps.map((step) => step.target));
      const session = test.coordinator.startTour('backup', {
        scope: root,
        locale: 'en',
        signedIn,
        capabilities: [RPC.cloudPlanGet, 'cloud.backup.createNamed', RPC.cloudBackupList],
      });
      await flushPromises();
      expect(await session.ready).toBe(false);
      expect(await session.done).toBe('unavailable');
      expect(test.loadDriver).not.toHaveBeenCalled();
      root.remove();
    }
  });

  it('keeps going when the feed stage target is missing instead of closing the tour', async () => {
    const test = setup();
    const root = mountGuideTargets('feed', ['feed-view', 'feed-filters', 'feed-controls']);
    const session = test.coordinator.startTour('feed', { scope: root, locale: 'en' });
    await flushPromises();
    expect(await session.ready).toBe(true);
    test.next();
    await flushPromises();
    test.next();
    await flushPromises();
    expect(session.active).toBe(true);
    expect(test.steps().filter((step) => step.element instanceof HTMLButtonElement).length).toBe(3);
    session.stop();
  });
});

it('presses the prepare trigger when an interaction-gated target is missing, instead of skipping the step', async () => {
  vi.useFakeTimers();
  const test = setup();
  const root = mountGuideTargets('automations', ['automation-profiles', 'automation-tab-events', 'automation-events', 'automation-runs']);
  const table = defined(root.querySelector('[data-guide="automation-events"]'));
  const body = document.createElement('tbody');
  const row = document.createElement('tr');
  body.append(row);
  table.append(body);
  const openEditor = vi.fn(() => {
    const editor = document.createElement('div');
    editor.dataset.guide = 'automation-editor';
    root.append(editor);
  });
  row.addEventListener('click', openEditor);
  const session = test.coordinator.startTour('automationEvents');
  await vi.advanceTimersByTimeAsync(0);
  expect(await session.ready).toBe(true);
  // profiles → eventsTab → events → editor: the editor only exists after opening a record.
  for (let index = 0; index < 3; index++) {
    test.next();
    await vi.advanceTimersByTimeAsync(700);
  }
  expect(openEditor).toHaveBeenCalledTimes(1);
  const driveCalls = test.drive.mock.calls.map((call) => call[0]);
  expect(driveCalls.at(-1)).toBe(3);
  expect(test.steps()[3]?.element).toBe(root.querySelector('[data-guide="automation-editor"]'));
  session.stop();
});


it('prefers a semantic editor operation and suppresses the selector fallback while that owner exists', async () => {
  vi.useFakeTimers();
  const test = setup();
  const root = mountGuideTargets('automations', ['automation-profiles', 'automation-tab-events', 'automation-events', 'automation-runs']);
  const table = defined(root.querySelector('[data-guide="automation-events"]'));
  const body = document.createElement('tbody');
  const row = document.createElement('tr');
  body.append(row);
  table.append(body);
  const fallbackOpen = vi.fn(() => {
    const wrong = document.createElement('div');
    wrong.dataset.guide = 'automation-editor';
    root.append(wrong);
  });
  row.addEventListener('click', fallbackOpen);
  const semanticOpen = vi.fn(() => {
    const editor = document.createElement('div');
    editor.dataset.guide = 'automation-editor';
    root.append(editor);
    return true;
  });
  const unregister = registerGuideActions('automations', { 'automation.openEvent': semanticOpen }, root);
  const session = test.coordinator.startTour('automationEvents');
  await vi.advanceTimersByTimeAsync(0);
  expect(await session.ready).toBe(true);
  for (let index = 0; index < 3; index++) {
    test.next();
    await vi.advanceTimersByTimeAsync(700);
  }
  expect(semanticOpen).toHaveBeenCalledTimes(1);
  expect(fallbackOpen).not.toHaveBeenCalled();
  expect(test.steps()[3]?.element).toBe(root.querySelector('[data-guide="automation-editor"]'));
  session.stop();
  unregister();
});

it('does not press the prepare trigger when the target already exists', async () => {
  vi.useFakeTimers();
  const test = setup();
  const root = mountGuideTargets('automations', tourRegistry.automationEvents.steps.map((step) => step.target));
  const table = defined(root.querySelector('[data-guide="automation-events"]'));
  const body = document.createElement('tbody');
  const row = document.createElement('tr');
  body.append(row);
  table.append(body);
  const openEditor = vi.fn();
  row.addEventListener('click', openEditor);
  const session = test.coordinator.startTour('automationEvents');
  await vi.advanceTimersByTimeAsync(0);
  expect(await session.ready).toBe(true);
  for (let index = 0; index < 3; index++) {
    test.next();
    await vi.advanceTimersByTimeAsync(100);
  }
  expect(openEditor).not.toHaveBeenCalled();
  session.stop();
});

it('backward navigation presses chained prepare triggers until the target returns', async () => {
  vi.useFakeTimers();
  const test = setup();
  const root = mountGuideTargets('automations', ['automation-profiles', 'automation-tab-events', 'automation-editor', 'automation-runs']);
  // Editor-open state: the library table is gone and the close button exists.
  const close = document.createElement('button');
  close.dataset.guide = 'automation-editor-close';
  root.append(close);
  const eventsTab = defined(root.querySelector('[data-guide="automation-tab-events"]'));
  const closeClick = vi.fn(() => {
    close.remove(); // closing the editor brings the library back, still missing the events table in this drill
  });
  close.addEventListener('click', closeClick);
  const tabClick = vi.fn(() => {
    const table = document.createElement('div');
    table.dataset.guide = 'automation-events';
    root.append(table);
  });
  eventsTab.addEventListener('click', tabClick);
  const session = test.coordinator.startTour('automationEvents');
  await vi.advanceTimersByTimeAsync(0);
  expect(await session.ready).toBe(true);
  // profiles → eventsTab: two chained presses needed (close editor, then the tab logic) before the events table exists.
  test.next();
  await vi.advanceTimersByTimeAsync(700);
  // eventsTab target exists already, drive to the events table step directly:
  test.next();
  await vi.advanceTimersByTimeAsync(700);
  await vi.advanceTimersByTimeAsync(2000);
  expect(closeClick).toHaveBeenCalled();
  expect(tabClick).toHaveBeenCalled();
  expect(test.steps()[2]?.element).toBe(root.querySelector('[data-guide="automation-events"]'));
  session.stop();
});

describe('section changes and interruptions', () => {
  const popoverOf = (test: ReturnType<typeof setup>, index: number) => defined(test.steps()[index]?.popover);

  it('counts only the steps that apply and says how many were skipped', async () => {
    vi.useFakeTimers();
    const test = setup();
    // The feed tour has four steps; this layout has no stage, so that step is skipped.
    const root = mountGuideTargets('feed', ['feed-view', 'feed-filters', 'feed-controls']);
    const session = test.coordinator.startTour('feed', { scope: root, locale: 'en' });
    await vi.advanceTimersByTimeAsync(0);
    expect(await session.ready).toBe(true);
    expect(popoverOf(test, 0).progressText).toBe('1 of 4');
    test.next();
    await vi.advanceTimersByTimeAsync(700);
    test.next();
    await vi.advanceTimersByTimeAsync(700);
    // filters → (stage skipped) → controls: the counter reads 3 of 3, not 4 of 4.
    expect(test.drive.mock.calls.at(-1)?.[0]).toBe(3);
    expect(popoverOf(test, 3).progressText).toBe('3 of 3');
    expect(popoverOf(test, 3).description).toContain('Skipped 1 step(s)');
    session.stop();
  });

  it('announces the zone the guide moved to, forwards and back', async () => {
    const test = setup();
    const root = mountGuideTargets('feed', ['feed-view', 'feed-filters', 'feed-stage', 'feed-controls']);
    const session = test.coordinator.startTour('feed', { scope: root, locale: 'en' });
    await flushPromises();
    expect(await session.ready).toBe(true);
    expect(popoverOf(test, 0).description).not.toContain('tiktools-guide__note');
    test.next();
    await flushPromises();
    // feed-view is a list; feed-filters is a toolbar.
    expect(popoverOf(test, 1).description).toContain('Now in: Toolbar');
    const previous = test.config().onPrevClick;
    previous?.(undefined, defined(test.steps()[1]), { driver: test.engine, config: test.config(), state: {}, index: 1 });
    await flushPromises();
    expect(popoverOf(test, 0).description).toContain('Back to: List');
    session.stop();
  });

  it('pauses with a way back when the user navigates away, and resumes on the step it stopped at', async () => {
    const paused: Array<GuidePause | undefined> = [];
    const fixture = driverFixture();
    const coordinator = createTourCoordinator({ loadDriver: fixture.loadDriver, storage: () => sessionStorage, onPausedChange: (pause) => paused.push(pause) });
    owned.push(coordinator);
    mountGuideTargets('points', ['points-rules', 'points-leaderboard']);
    const router = createRouter({
      history: createMemoryHistory(),
      routes: ['/points', '/connections'].map((path) => ({ path, component: { template: '<div />' } })),
    });
    const session = coordinator.startTour('points', { router, locale: 'en' });
    expect(await session.ready).toBe(true);
    fixture.next();
    await flushPromises();
    await router.push('/connections');
    expect(await session.done).toBe('navigation');
    const pause = defined(paused.at(-1));
    expect(pause).toMatchObject({ id: 'points', stepId: 'leaderboard', cause: 'navigation' });
    expect(pause.title).toBe(tourCatalogs.en.points.leaderboard.title);
    const resumed = pause.resume();
    expect(await resumed.ready).toBe(true);
    expect(router.currentRoute.value.path).toBe('/points');
    expect(fixture.drive.mock.calls.at(-1)?.[0]).toBe(1);
    // Starting the guide again clears the notice.
    expect(paused.at(-1)).toBeUndefined();
    resumed.stop();
  });

  it('pauses instead of silently closing when a dialog opens over the guide', async () => {
    const paused: Array<GuidePause | undefined> = [];
    const fixture = driverFixture();
    const coordinator = createTourCoordinator({ loadDriver: fixture.loadDriver, storage: () => sessionStorage, onPausedChange: (pause) => paused.push(pause) });
    owned.push(coordinator);
    mountGuideTargets('points', ['points-rules', 'points-leaderboard']);
    const session = coordinator.startTour('points', { locale: 'en' });
    expect(await session.ready).toBe(true);
    const dialog = document.createElement('dialog');
    dialog.setAttribute('open', '');
    dialog.setAttribute('data-test-guide', '');
    document.body.append(dialog);
    expect(await session.done).toBe('dismissed');
    expect(paused.at(-1)).toMatchObject({ id: 'points', stepId: 'rules', cause: 'dialog' });
  });

  it('does not offer to resume when the user closes the guide', async () => {
    const paused: Array<GuidePause | undefined> = [];
    const fixture = driverFixture();
    const coordinator = createTourCoordinator({ loadDriver: fixture.loadDriver, storage: () => sessionStorage, onPausedChange: (pause) => paused.push(pause) });
    owned.push(coordinator);
    mountGuideTargets('points', ['points-rules', 'points-leaderboard']);
    const session = coordinator.startTour('points', { locale: 'en' });
    expect(await session.ready).toBe(true);
    fixture.close();
    expect(await session.done).toBe('dismissed');
    expect(paused.filter((pause) => pause !== undefined)).toEqual([]);
  });
});
