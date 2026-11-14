import { createApp, h, reactive, watch } from 'vue';
import DemoToolbar from './DemoToolbar.vue';
import { createControlClient } from '@/shared/api/control-client.ts';
import { mountWorkspace } from '@/app/mount-workspace.ts';
import { createDemoHost } from './runtime/host.ts';
import { validDemoRoute, type parseDemoOptions, type DemoLocale } from './scenarios.ts';
import '@/app/styles/tokens.css';
import '@/app/styles/base.css';
import '@/app/styles/responsive.css';
import { startTour, stopTour, tourRegistry, type GuideSession } from '@/features/guided-help/index.ts';
import { configureAppClock } from '@/shared/lib/app-clock.ts';
import { DEMO_AT } from './fixtures/scenario-state.ts';
import { diagnosticLog, diagnosticView } from '@/shared/diagnostics/log.ts';
import { toolbarCopy } from './toolbar-copy.ts';
import { createDemoPlayback, type PlaybackState } from './playback.ts';
import { demoTourScenarios, demoTourUrl, scenarioTour } from './tours.ts';
import { isTourId } from '@/features/guided-help/model/registry.ts';
import type { GuideOutcome } from '@/features/guided-help/model/types.ts';
import { experienceCopy } from '../../../docs/experience/copy.ts';
import { useNavigationStore } from '@/shared/model/navigation-store.ts';
import type { BuiltinAppTab } from '@/types.ts';
import { parseFrameCommand, type ConnectionCase, type DemoFrameState } from './frame-messages.ts';
export async function bootDemo(options: ReturnType<typeof parseDemoOptions>, storage: { clear(): void }) {
  const host = createDemoHost(options.scenario, options.locale);
  configureAppClock(() => DEMO_AT + host.diagnostics().ticks * 1000);
  const client = createControlClient({ transport: host.transport });
  if (options.embed || !validDemoRoute(location.hash.slice(1))) location.hash = options.route;
  const recordFailure = () => diagnosticLog.record({ source: 'integrations', level: 'error', code: 'demo.simulated_action_failure', message: ({ en: 'Simulated action failed. Inspect the run result, then retry or reset.', es: 'Falló una acción simulada. Revisa el resultado y reintenta o reinicia.', pt: 'Uma ação simulada falhou. Confira o resultado e tente novamente ou reinicie.', fr: 'Une action simulée a échoué. Consultez le résultat, puis réessayez ou réinitialisez.', zh: '模拟操作失败。请检查执行结果，然后重试或重置。', ja: 'サンプルのアクションが失敗しました。実行結果を確認し、再試行するかリセットしてください。' })[options.locale], requestId: 'demo-request' });
  if (options.scenario === 'diagnostics') recordFailure();
  const toolbar = document.querySelector('#demo-controls')!;
  const toolbarState = reactive({ paused: host.paused, busy: false, events: 0, status: 'ready' as PlaybackState['status'], guideState: 'idle', failure: false, ready: false, metrics: '', connectionCase: 'normal' as 'normal' | 'offline' | 'auth' });
  function publishFrameState() {
    if (!options.framed || window.parent === window) return;
    const state: DemoFrameState = { type: 'tiktools.demo.state', instance: options.instance, ready: toolbarState.ready,
      paused: toolbarState.paused, busy: toolbarState.busy || ['preparing', 'active'].includes(toolbarState.guideState),
      events: toolbarState.events, status: toolbarState.status, connectionCase: toolbarState.connectionCase };
    window.parent.postMessage(state, location.origin);
  }
  const playback = createDemoPlayback(host, client, (state) => { Object.assign(toolbarState, state); publishFrameState(); });
  async function changeConnectionCase(kind: ConnectionCase) {
    playback.pause();
    await client.call('live.disconnect', {});
    toolbarState.connectionCase = kind;
    const copy = experienceCopy[options.locale].connectionStates;
    host.setConnectionExample(kind, kind === 'offline' ? copy.offlineHelp : kind === 'auth' ? copy.authHelp : '');
    publishFrameState();
  }
  const tours = (Object.keys(tourRegistry) as Array<keyof typeof tourRegistry>).map((value) => ({ value, label: toolbarCopy[options.locale].tourTitles[value] }));
  const initialTour = options.tour ?? scenarioTour(options.scenario);
  const tourActions: { start?: (id: string, trigger: HTMLElement) => void; stop?: () => void } = {};
  const onScenario = (scenario: string) => { const url = new URL(location.href); url.searchParams.set('scenario', scenario); url.searchParams.delete('route'); url.searchParams.delete('tour'); url.hash = ''; location.assign(url.href); };
  const onLocale = (locale: DemoLocale) => { const url = new URL(location.href); url.searchParams.set('locale', locale); location.assign(url.href); };
  const toolbarApp = createApp({ setup: () => () => h(DemoToolbar, {
    ...toolbarState, scenario: options.scenario, locale: options.locale, initialTour, tours, embed: options.embed,
    docsUrl: new URL(`../${options.locale}/`, location.href).href,
    onScenario, onLocale,
    onPlay: () => { void playback.play(); },
    onStep: () => { void playback.step(); }, onReset: resetDemo,
    onFailure: () => { toolbarState.failure = !toolbarState.failure; host.simulateError(toolbarState.failure); if (toolbarState.failure) recordFailure(); },
    onCatalog: () => { location.hash = '/components/buttons'; },
    onGuide: (id: string, trigger: HTMLElement) => tourActions.start?.(id, trigger),
    onCancelGuide: () => tourActions.stop?.(),
    onConnectionCase: changeConnectionCase,
  }) });
  toolbarApp.config.idPrefix = 'demo-tools';
  toolbarApp.mount(toolbar);
  const app = await mountWorkspace({ client, locale: options.locale, configure(app) {
    const router = app.config.globalProperties.$router;
    router.beforeEach((to) => options.embed && to.path !== options.route && to.path !== '/debug' ? false : validDemoRoute(to.fullPath) ? true : '/feed');
    if (options.embed) {
      // Real connection actions normally open the feed. A focused lesson keeps
      // its panel visible so successes and failures can be inspected in place.
      const tabs: Record<NonNullable<typeof options.embed>, BuiltinAppTab> = { connection: 'connect', feed: 'feed', automation: 'behavior', points: 'points', overlays: 'widgets' };
      const target = tabs[options.embed];
      const navigation = app.runWithContext(() => useNavigationStore());
      navigation.open(target);
      const stop = watch(() => navigation.activeTab, (tab) => { if (tab !== target) navigation.open(target); }, { flush: 'sync' });
      app.onUnmount(stop);
    }
  } }).catch((error: unknown) => { playback.dispose(); toolbarApp.unmount(); client.detach(); host.dispose(); configureAppClock(undefined); throw error; });
  const discovery = await client.call<{ methods: Array<{ name: string }> }>('rpc.discover', {});
  let tour: GuideSession | undefined;
  const taskState = () => ({
    'connection.connected': host.state.liveStatus.connected,
    'automation.saved': host.wasSuccessful('automation.create') || host.wasSuccessful('automation.update'),
    'plugin.installed': host.wasSuccessful('plugins.install.set') && host.pluginInstalled(),
    'overlay.previewed': host.wasSuccessful('widgets.copyObsUrl') || host.wasSuccessful('bundles.overlays.create'),
    'points.reviewed': host.wasSuccessful('points.config.set') || host.wasSuccessful('points.adjust'),
    'changes.reviewed': host.state.variableComparisons?.length === 0,
    'diagnostics.inspected': diagnosticLog.entries.value.some((entry) => entry.id === diagnosticView.selectedId.value),
  });
  const outcomeState = (outcome: GuideOutcome) => outcome === 'completed' || outcome === 'task-completed' ? 'completed' : outcome === 'unavailable' ? 'unavailable' : 'dismissed';
  const launchTour = (id: string, trigger?: HTMLElement) => {
    if (!isTourId(id)) return;
    playback.pause();
    if (demoTourScenarios[id] !== options.scenario) {
      location.assign(demoTourUrl(location.href, id));
      return;
    }
    tour?.stop('replaced');
    toolbarState.guideState = 'preparing';
    const session = startTour(id, { router: app.config.globalProperties.$router, capabilities: discovery.methods.map((method) => method.name), locale: options.locale, trigger, taskState });
    tour = session;
    void session.ready.then((active) => { if (tour === session && active) toolbarState.guideState = 'active'; });
    void session.done.then((outcome) => { if (tour === session) { toolbarState.guideState = outcomeState(outcome); tour = undefined; } });
  };
  tourActions.start = (id, trigger) => {
    if (tour?.id === id && toolbarState.guideState === 'active') tour.stop('dismissed');
    else launchTour(id, trigger);
  };
  tourActions.stop = () => { tour?.stop('dismissed'); tour = undefined; toolbarState.guideState = 'idle'; };

  toolbarState.ready = true;
  if (options.embed === 'feed') for (let index = 0; index < options.initialEvents; index += 1) await playback.step();
  if (options.tour) launchTour(options.tour);
  function onFrameCommand(event: MessageEvent) {
    if (!options.framed || !toolbarState.ready || event.origin !== location.origin || event.source !== window.parent) return;
    const command = parseFrameCommand(event.data, options.instance);
    if (!command) return;
    if (command.action === 'reset') resetDemo();
    else if (command.action === 'connection-case') void changeConnectionCase(command.value);
    else if (!toolbarState.busy && !['preparing', 'active'].includes(toolbarState.guideState)) {
      if (command.action === 'play') void playback.play();
      else void playback.step();
    }
  }
  if (options.framed) window.addEventListener('message', onFrameCommand);
  publishFrameState();
  const timer = setInterval(() => playback.tick(), 2000);
  // Observable state has no mutators or test-only globals, and is useful for support diagnostics.
  const metricsTimer = setInterval(() => { toolbarState.paused = host.paused; toolbarState.metrics = JSON.stringify({ host: host.diagnostics(), client: client.diagnostics?.() }); publishFrameState(); }, 500);
  let disposed = false;
  function cleanup() { if (disposed) return; disposed = true; tour?.stop('disposed'); stopTour(); playback.dispose(); clearInterval(timer); clearInterval(metricsTimer); toolbarApp.unmount(); client.detach(); host.dispose(); configureAppClock(undefined); window.removeEventListener('message', onFrameCommand); window.removeEventListener('pagehide', onPageHide); window.removeEventListener('pageshow', restore); }
  function dispose() { if (disposed) return; cleanup(); app.unmount(); }
  function resetDemo() {
    // Stop pending preference writes before clearing this instance's storage.
    dispose();
    storage.clear();
    location.reload();
  }
  function restore(event: PageTransitionEvent) { if (event.persisted) location.reload(); }
  function onPageHide(event: PageTransitionEvent) { dispose(); if (event.persisted) window.addEventListener('pageshow', restore, { once: true }); }
  window.addEventListener('pagehide', onPageHide);
  app.onUnmount(cleanup);
  return { app, host, dispose };
}
