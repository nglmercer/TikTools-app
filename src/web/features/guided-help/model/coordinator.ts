import { watch } from 'vue';
import { appStorage } from '@/shared/lib/app-storage.ts';
import { i18n } from '@/shared/i18n/i18n.ts';
import { isLocale } from '@/shared/i18n/supported-locales.ts';
import type { Config, Driver } from 'driver.js';
import { tourCatalogs, type TourCatalog } from '../catalogs/index.ts';
import { isTourId, tourRegistry } from './registry.ts';
import { readTourProgress, writeTourProgress } from './progress.ts';
import { hasGuideAction, runGuideAction } from './guide-actions.ts';
import { guideScopeSelector, guideTargetSelector, isVisible, queryGuide } from './guide-dom.ts';
import { automationActionExampleAvailable, automationActionTypeAvailable } from './tour-requirements.ts';
import type { GuideOutcome, GuideSession, GuideStep, StartTourOptions, TourDefinition, TourLocale, TourStepDefinition, TourTaskState } from './types.ts';

type DriverFactory = (config: Config) => Driver;
export interface CoordinatorDependencies {
  loadDriver?: () => Promise<{ driver: DriverFactory }>;
  storage?: () => Storage | undefined;
  locale?: () => TourLocale;
}
export interface InlineGuideOptions {
  id: string;
  steps: () => readonly GuideStep[];
  trigger?: HTMLElement;
  scope?: Element;
  completed?: () => boolean;
  labels?: TourCatalog['controls'];
  onActiveChange?: (active: boolean) => void;
}
interface RunStep extends GuideStep { version?: number; missingTargetPolicy?: 'skip' | 'cancel'; prepareClick?: readonly string[]; prepareActions?: readonly string[]; popoverSide?: 'top' | 'right' | 'bottom' | 'left'; popoverAlign?: 'start' | 'center' | 'end'; available?: () => boolean; availabilityPending?: () => boolean }
interface RunOptions extends StartTourOptions {
  id: string;
  version?: number;
  route?: string;
  scopeName?: string;
  task?: string;
  steps: () => readonly RunStep[];
  completed?: () => boolean;
  labels: TourCatalog['controls'];
  onActiveChange?: (active: boolean) => void;
}

export const MAX_GUIDE_WAIT_MS = 5_000;
const DEFAULT_WAIT_MS = 2_000;
const escaped = (value: string) => value.replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character]!);
const modalOpen = () => document.querySelector('dialog[open], [role="dialog"][aria-modal="true"]');

/** One observer, one deadline and one abort listener per wait, all removed on every exit. */
function waitFor<T>(read: () => T | undefined, signal: AbortSignal, timeout: number): Promise<T | undefined> {
  if (signal.aborted) return Promise.resolve(undefined);
  const immediate = read();
  if (immediate !== undefined) return Promise.resolve(immediate);
  return new Promise((resolve) => {
    let finished = false;
    const finish = (result?: T) => {
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      clearInterval(poll);
      observer.disconnect();
      signal.removeEventListener('abort', cancel);
      resolve(result);
    };
    const cancel = () => finish();
    const check = () => {
      const result = read();
      if (result !== undefined) finish(result);
    };
    const observer = new MutationObserver(check);
    const timer = setTimeout(cancel, timeout);
    // Layout and CSS transitions can become visible without another DOM mutation.
    const poll = setInterval(check, 50);
    signal.addEventListener('abort', cancel, { once: true });
    observer.observe(document.body, { childList: true, subtree: true, attributes: true });
  });
}

function within<T>(work: Promise<T>, signal: AbortSignal, timeout: number): Promise<T | undefined> {
  return new Promise((resolve, reject) => {
    let finished = false;
    const finish = (result?: T, error?: unknown) => {
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      signal.removeEventListener('abort', cancel);
      if (error !== undefined) reject(error);
      else resolve(result);
    };
    const cancel = () => finish();
    const timer = setTimeout(cancel, timeout);
    signal.addEventListener('abort', cancel, { once: true });
    if (signal.aborted) cancel();
    void work.then((result) => finish(result), (error: unknown) => finish(undefined, error));
  });
}

async function loadDriver() {
  const [engine] = await Promise.all([import('driver.js'), import('driver.js/dist/driver.css'), import('../ui/guide.css')]);
  return engine;
}

export interface DynamicTour {
  definition: TourDefinition;
  texts: Readonly<Record<string, string>>;
}

export function createTourCoordinator(dependencies: CoordinatorDependencies = {}) {
  let current: GuideSession | undefined;
  const storage = () => {
    try { return dependencies.storage ? dependencies.storage() : appStorage('session'); }
    catch { return undefined; }
  };
  const locale = (): TourLocale => {
    const value = dependencies.locale?.() ?? i18n.global.locale.value;
    return isLocale(value) ? value : 'en';
  };

  function run(options: RunOptions): GuideSession {
    current?.stop('replaced');
    const abort = new AbortController();
    const waitMs = Math.min(MAX_GUIDE_WAIT_MS, Math.max(0, Number.isFinite(options.waitMs) ? options.waitMs! : DEFAULT_WAIT_MS));
    const progressStorage = storage();
    let engine: Driver | undefined;
    let observer: MutationObserver | undefined;
    let removeNavigation: (() => void) | undefined;
    let stepAbort: AbortController | undefined;
    let scope: Element | Document | undefined;
    let finished = false;
    let active = false;
    let moving = false;
    let moveGeneration = 0;
    let activeIndex = -1;
    let lastStepId = '';
    let reachedFinalStep = false;
    let suppliedTaskState: TourTaskState | undefined;
    let resolveReady!: (active: boolean) => void;
    let resolveDone!: (outcome: GuideOutcome) => void;
    const ready = new Promise<boolean>((resolve) => { resolveReady = resolve; });
    const done = new Promise<GuideOutcome>((resolve) => { resolveDone = resolve; });
    const requiresVerification = options.completed !== undefined || options.task !== undefined;
    const taskComplete = () => {
      try {
        const state = suppliedTaskState ?? (typeof options.taskState === 'function' ? options.taskState() : options.taskState);
        return options.completed?.() === true || (options.task !== undefined && state?.[options.task] === true);
      } catch { return false; }
    };
    const completionAccepted = () => taskComplete() || (!requiresVerification && reachedFinalStep);
    function save(status: 'in-progress' | 'dismissed' | 'completed'): void {
      if (!lastStepId && status === 'completed') lastStepId = options.steps().at(-1)?.id ?? '';
      if (!lastStepId) return;
      if (options.version !== undefined) {
        const previous = readTourProgress(progressStorage, options.id, options.version, options.steps().map((step) => step.id));
        writeTourProgress(progressStorage, options.id, { version: options.version, stepId: lastStepId, status: previous?.status === 'completed' ? 'completed' : status });
      } else {
        try { progressStorage?.setItem(`tiktools.guide.${options.id}`, lastStepId); } catch { /* Optional legacy step memory. */ }
      }
    }
    function finish(reason: GuideOutcome, destroy = true): void {
      if (finished) return;
      const verified = completionAccepted();
      const outcome = (reason === 'completed' || reason === 'task-completed') && !verified ? 'dismissed' : reason;
      finished = true;
      stepAbort?.abort();
      abort.abort();
      observer?.disconnect();
      removeNavigation?.();
      stopTaskWatch?.();
      save((outcome === 'completed' || outcome === 'task-completed') && verified ? 'completed' : 'dismissed');
      active = false;
      options.onActiveChange?.(false);
      const previous = engine;
      engine = undefined;
      if (destroy) previous?.destroy();
      if (current === session) current = undefined;
      if (options.trigger?.isConnected && !modalOpen()) options.trigger.focus({ preventScroll: true });
      resolveReady(false);
      resolveDone(outcome);
    }
    function target(step: GuideStep): HTMLElement | undefined {
      if (!scope || (scope instanceof Element && !scope.isConnected)) return;
      const element = queryGuide(scope, step.selector);
      return isVisible(element) ? element : undefined;
    }
    function savedStep(steps: readonly GuideStep[]): string {
      if (options.version !== undefined) return readTourProgress(progressStorage, options.id, options.version, steps.map((step) => step.id))?.stepId ?? '';
      try { return progressStorage?.getItem(`tiktools.guide.${options.id}`) ?? ''; } catch { return ''; }
    }
    function handleMissing(): void {
      if (finished || moving) return;
      const step = options.steps()[activeIndex];
      if (options.version !== undefined && step?.missingTargetPolicy === 'skip') void moveToStep(activeIndex + 1);
      else finish('unavailable');
    }
    function installObserver(): void {
      observer = new MutationObserver(() => {
        if (finished) return;
        if (modalOpen()) { finish('dismissed'); return; }
        if (options.version === undefined && taskComplete()) { finish('task-completed'); return; }
        if (scope instanceof Element && !scope.isConnected) { finish('unavailable'); return; }
        const element = engine?.getActiveElement?.();
        if (element && !isVisible(element)) handleMissing();
      });
      observer.observe(document.body, { childList: true, subtree: true, attributes: true, attributeFilter: ['open', 'hidden', 'disabled', 'aria-hidden', 'aria-disabled', 'aria-modal', 'inert', 'style', 'class'] });
    }
    async function ensureEngine(): Promise<boolean> {
      if (engine) return true;
      const module = await within((dependencies.loadDriver ?? loadDriver)(), abort.signal, waitMs);
      if (!module || finished) { finish('unavailable'); return false; }
      engine = module.driver({
        animate: !(typeof window.matchMedia === 'function' && window.matchMedia('(prefers-reduced-motion: reduce)').matches),
        smoothScroll: false, allowClose: true, allowKeyboardControl: true, popoverClass: 'tiktools-guide',
        allowedElements: scope instanceof Element ? [scope] : [],
        nextBtnText: options.labels.next, prevBtnText: options.labels.previous, doneBtnText: options.labels.done,
        closeBtnLabel: options.labels.close, progressText: options.labels.progress,
        onHighlighted: (_element, step) => { lastStepId = String(step.data?.id ?? ''); save('in-progress'); },
        onNextClick: () => {
          if (options.version !== undefined) { if (!moving) void moveToStep(activeIndex + 1); }
          else if (engine?.isLastStep()) { reachedFinalStep = true; finish(completionAccepted() ? 'completed' : 'dismissed'); }
          else engine?.moveNext();
        },
        onDoneClick: () => {
          if (options.version !== undefined && activeIndex < options.steps().length - 1) { if (!moving) void moveToStep(activeIndex + 1); }
          else { reachedFinalStep = true; finish(completionAccepted() ? 'completed' : 'dismissed'); }
        },
        onPrevClick: () => {
          if (options.version !== undefined) { if (!moving) void moveToStep(activeIndex - 1, -1); }
          else engine?.movePrevious();
        },
        onCloseClick: () => finish('dismissed'),
        onDestroyed: () => finish('dismissed', false),
        onElementMissing: () => { handleMissing(); return false; },
      });
      installObserver();
      return true;
    }
    /** Legacy button props retain immediate filtering and their single-step close-only UI. */
    function showInline(): void {
      if (!engine || finished) return;
      if (taskComplete()) { finish('task-completed'); return; }
      if (modalOpen()) { finish('dismissed'); return; }
      const steps = options.steps().flatMap((step) => {
        const element = target(step);
        return element ? [{ ...step, element }] : [];
      });
      if (!steps.length) { finish('unavailable'); return; }
      lastStepId ||= savedStep(steps);
      engine.setSteps(steps.map((step) => ({ element: step.element, data: { id: step.id }, popover: { title: escaped(step.title), description: escaped(step.description) } })));
      engine.setConfig({ ...engine.getConfig(), showProgress: steps.length > 1, showButtons: steps.length > 1 ? ['next', 'previous', 'close'] : ['close'] });
      const index = Math.max(0, steps.findIndex((step) => step.id === lastStepId));
      lastStepId = steps[index]!.id;
      engine.drive(index);
      resolveReady(active);
    }
    /** Each selected step gets its own visibility deadline and explicit missing-target outcome. */
    async function moveToStep(first: number, direction = 1): Promise<void> {
      stepAbort?.abort();
      const ticket = ++moveGeneration;
      const pending = new AbortController();
      stepAbort = pending;
      moving = true;
      try {
        const steps = options.steps();
        for (let index = first; index >= 0 && index < steps.length; index += direction) {
          if (finished || ticket !== moveGeneration) return;
          const step = steps[index]!;
          // Availability has two failure kinds. Static failures (signed-out
          // account, denied capability) never resolve and the step skips
          // instantly. Requirement failures can be async-pending: the
          // automation behavior snapshot backing them loads after page mount,
          // and skipping in the milliseconds before it arrives killed every
          // requirement-bearing tour before its first popover. Only the
          // pending kind gets the wait deadline targets already had.
          if (step.available?.() === false) {
            if (!step.availabilityPending?.()) continue;
            const available = await waitFor(() => step.available!() ? true : undefined, pending.signal, Math.min(waitMs, 600));
            if (!available) continue;
          }
          let element = await waitFor(() => target(step), pending.signal, Math.min(waitMs, 600));
          // Targets can be several interactions away. Prefer semantic operations
          // registered by the page: they know whether closing an editor is safe,
          // whether a row exists, and which tab is active. Selector clicks remain
          // only as a non-mutating fallback for embeds/legacy fixtures.
          for (let round = 0; !element && (step.prepareActions?.length || step.prepareClick?.length) && scope && round < 5; round++) {
            let acted = false;
            for (const action of step.prepareActions ?? []) {
              if (await runGuideAction(scope, options.scopeName, action, step.selector)) { acted = true; break; }
            }
            const semanticOwner = (step.prepareActions ?? []).some((action) => hasGuideAction(options.scopeName, action, scope));
            if (!acted && !semanticOwner) {
              for (const selector of step.prepareClick ?? []) {
                const trigger = queryGuide(scope, selector);
                if (isVisible(trigger)) { trigger.click(); acted = true; break; }
              }
            }
            if (!acted) break;
            element = await waitFor(() => target(step), pending.signal, waitMs);
          }
          if (finished || pending.signal.aborted || ticket !== moveGeneration) return;
          if (!element) {
            if (step.missingTargetPolicy === 'cancel') { finish('unavailable'); return; }
            continue;
          }
          if (modalOpen()) { finish('dismissed'); return; }
          if (!await ensureEngine() || finished || ticket !== moveGeneration) return;
          // Inactive missing steps use detached sentinels, never a global selector or another workspace.
          engine!.setSteps(steps.map((item) => ({
            element: target(item) ?? document.createElement('span'),
            data: { id: item.id, version: item.version },
            popover: {
              title: escaped(item.title), description: escaped(item.description),
              ...(item.popoverSide ? { side: item.popoverSide } : {}),
              ...(item.popoverAlign ? { align: item.popoverAlign } : {}),
            },
          })));
          engine!.setConfig({ ...engine!.getConfig(), showProgress: steps.length > 1, showButtons: ['next', 'previous', 'close'] });
          activeIndex = index;
          lastStepId = step.id;
          active = true;
          options.onActiveChange?.(true);
          engine!.drive(index);
          resolveReady(true);
          return;
        }
        if (direction > 0) {
          if (active) reachedFinalStep = true;
          finish(active ? completionAccepted() ? 'completed' : 'dismissed' : 'unavailable');
        }
      } catch { finish('unavailable'); }
      finally { if (ticket === moveGeneration) moving = false; }
    }
    function refresh(): void {
      if (finished) return;
      if (taskComplete()) { finish('task-completed'); return; }
      if (!engine) return;
      if (options.version === undefined) showInline();
      else void moveToStep(Math.max(0, activeIndex));
    }
    const session: GuideSession = {
      id: options.id, ready, done,
      get active() { return active; },
      stop: (reason = 'dismissed') => finish(reason),
      refresh,
      updateTaskState: (state) => { suppliedTaskState = state; refresh(); },
    };
    current = session;
    const stopTaskWatch = watch(taskComplete, (verified) => { if (verified) finish('task-completed'); });
    void (async () => {
      try {
        if (options.version === undefined && taskComplete()) { finish('task-completed'); return; }
        if (typeof document === 'undefined' || !document.body) { finish('unavailable'); return; }
        if (options.router) {
          const destination = options.route ? options.router.resolve(options.route).fullPath : options.router.currentRoute.value.fullPath;
          removeNavigation = options.router.afterEach((to) => { if (to.fullPath !== destination) finish('navigation'); });
          if (options.route && options.router.currentRoute.value.fullPath !== destination) {
            const moved = await within(options.router.push(options.route).then(() => true), abort.signal, waitMs);
            if (moved !== true || finished) { finish('unavailable'); return; }
          }
        }
        scope = await waitFor(() => {
          if (!options.scopeName) return options.scope?.isConnected ? options.scope : options.scope ? undefined : document;
          const found = queryGuide(options.scope ?? document, guideScopeSelector(options.scopeName));
          return found?.isConnected ? found : undefined;
        }, abort.signal, waitMs);
        if (!scope || finished) { finish('unavailable'); return; }
        if (options.version !== undefined) {
          const steps = options.steps();
          const saved = savedStep(steps);
          await moveToStep(Math.max(0, steps.findIndex((step) => step.id === saved)));
        } else {
          const found = await waitFor(() => options.steps().some((step) => target(step)) ? true : undefined, abort.signal, waitMs);
          if (!found || finished || modalOpen()) { finish('unavailable'); return; }
          if (!await ensureEngine()) return;
          active = true;
          options.onActiveChange?.(true);
          showInline();
        }
      } catch { finish('unavailable'); }
    })();
    return session;
  }

  function startDefinition(
    definition: TourDefinition,
    controls: TourCatalog['controls'],
    textAt: (key: string) => string,
    options: StartTourOptions,
  ): GuideSession {
    const existingScope = options.scope && queryGuide(options.scope, guideScopeSelector(definition.scope));
    const accountAvailable = () => definition.availability?.account !== 'signed-in'
      || (typeof options.signedIn === 'function' ? options.signedIn() : options.signedIn) === true;
    const snapshot = () => typeof options.automationSnapshot === 'function' ? options.automationSnapshot() : options.automationSnapshot;
    const requirementsAvailable = () => {
      const example = definition.requirements?.automationActionExample;
      const type = definition.requirements?.automationActionType;
      if (!example && !type) return true;
      const current = snapshot();
      if (!current) return false;
      return (!type || automationActionTypeAvailable(type, current)) &&
        (!example || automationActionExampleAvailable(example, current));
    };
    return run({
      ...options, id: definition.id, version: definition.version, route: existingScope?.isConnected ? undefined : definition.route, scopeName: definition.scope, task: definition.task,
      labels: controls,
      steps: () => (definition.steps as readonly TourStepDefinition[]).map((step) => ({
        id: step.id, version: step.version, selector: guideTargetSelector(step.target),
        title: textAt(step.titleKey), description: textAt(step.descriptionKey), missingTargetPolicy: step.missingTargetPolicy, prepareClick: step.prepareClick, prepareActions: step.prepareActions,
        popoverSide: step.popoverSide, popoverAlign: step.popoverAlign,
        available: () => accountAvailable() && requirementsAvailable() && step.requiredCapabilities.every((method) => !options.capabilities || ('supports' in options.capabilities ? options.capabilities.supports(method) !== false : options.capabilities.includes(method))),
        availabilityPending: () => accountAvailable()
          && step.requiredCapabilities.every((method) => !options.capabilities || ('supports' in options.capabilities ? options.capabilities.supports(method) !== false : options.capabilities.includes(method)))
          && !requirementsAvailable(),
      })),
    });
  }

  function startTour(id: string, options: StartTourOptions = {}): GuideSession {
    if (!isTourId(id)) throw new Error(`Unknown guided tour: ${id}`);
    const definition = tourRegistry[id];
    const catalog = tourCatalogs[options.locale ?? locale()];
    const textAt = (key: string): string => {
      let value: unknown = catalog;
      for (const part of key.split('.')) value = value && typeof value === 'object' ? (value as Record<string, unknown>)[part] : undefined;
      return typeof value === 'string' ? value : '';
    };
    return startDefinition(definition, catalog.controls, textAt, options);
  }

  function startDynamicTour(dynamic: DynamicTour, options: StartTourOptions = {}): GuideSession {
    const controls = tourCatalogs[options.locale ?? locale()].controls;
    return startDefinition(dynamic.definition, controls, (key) => dynamic.texts[key] ?? '', options);
  }
  return {
    startTour,
    startDynamicTour,
    startInlineGuide: (options: InlineGuideOptions) => run({ ...options, labels: options.labels ?? tourCatalogs[locale()].controls }),
    dismiss: () => current?.stop(),
    dispose: () => current?.stop('disposed'),
  };
}

let shared: ReturnType<typeof createTourCoordinator> | undefined;
const coordinator = () => shared ??= createTourCoordinator();
export const startTour = (id: string, options?: StartTourOptions) => coordinator().startTour(id, options);
export const startDynamicTour = (tour: DynamicTour, options?: StartTourOptions) => coordinator().startDynamicTour(tour, options);
export const startInlineGuide = (options: InlineGuideOptions) => coordinator().startInlineGuide(options);
export const stopTour = () => shared?.dismiss();
