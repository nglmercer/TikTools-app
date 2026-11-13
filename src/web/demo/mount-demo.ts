import { createApp, h, reactive } from 'vue';
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
import { tourCatalogs } from '@/features/guided-help/catalogs/index.ts';
export async function bootDemo(options: ReturnType<typeof parseDemoOptions>, storage: { clear(): void }) {
  const host = createDemoHost(options.scenario);
  configureAppClock(() => DEMO_AT + host.diagnostics().ticks * 1000);
  const client = createControlClient({ transport: host.transport });
  if (!validDemoRoute(location.hash.slice(1))) location.hash = options.route;
  const recordFailure = () => diagnosticLog.record({ source: 'transport', level: 'error', code: 'demo.simulated_disconnect', message: ({ en: 'Simulated connection interrupted. Retry or reset the demo.', es: 'Conexión simulada interrumpida. Reintenta o reinicia la demo.', pt: 'Conexão simulada interrompida. Tente novamente ou reinicie a demo.', fr: 'Connexion simulée interrompue. Réessayez ou réinitialisez la démo.' })[options.locale], requestId: 'demo-request' });
  if (options.scenario === 'diagnostics') recordFailure();
  const toolbar = document.querySelector('#demo-controls')!;
  const toolbarState = reactive({ paused: host.paused, failure: false, ready: false, metrics: '' });
  const tours = (Object.keys(tourRegistry) as Array<keyof typeof tourRegistry>).map((value) => ({ value, label: Object.values(tourCatalogs[options.locale][value])[0]!.title }));
  const initialTour = options.tour ?? ({ automation: 'automation', overlays: 'overlay', points: 'points', cloud: 'changes', diagnostics: 'diagnostics', plugins: 'plugin' } as Record<string, string>)[options.scenario] ?? 'connection';
  const tourActions: { start?: (id: string, trigger: HTMLElement) => void } = {};
  const onScenario = (scenario: string) => { const url = new URL(location.href); url.searchParams.set('scenario', scenario); url.searchParams.delete('route'); url.searchParams.delete('tour'); url.hash = ''; location.assign(url.href); };
  const onLocale = (locale: DemoLocale) => { const url = new URL(location.href); url.searchParams.set('locale', locale); location.assign(url.href); };
  const toolbarApp = createApp({ setup: () => () => h(DemoToolbar, {
    ...toolbarState, scenario: options.scenario, locale: options.locale, initialTour, tours,
    docsUrl: new URL(`../${options.locale === 'es' ? 'es' : 'en'}/`, location.href).href,
    onScenario, onLocale,
    onPlay: () => { host.setPaused(!host.paused); toolbarState.paused = host.paused; },
    onStep: () => host.step(), onReset: () => { storage.clear(); location.reload(); },
    onFailure: () => { toolbarState.failure = !toolbarState.failure; host.simulateError(toolbarState.failure); if (toolbarState.failure) recordFailure(); },
    onCatalog: () => { location.hash = '/ui-kit'; },
    onGuide: (id: string, trigger: HTMLElement) => tourActions.start?.(id, trigger),
  }) });
  toolbarApp.config.idPrefix = 'demo-tools';
  toolbarApp.mount(toolbar);
  const app = await mountWorkspace({ client, locale: options.locale, configure(app) {
    const router = app.config.globalProperties.$router;
    router.beforeEach((to) => validDemoRoute(to.fullPath) ? true : '/feed');
  } }).catch((error: unknown) => { toolbarApp.unmount(); client.detach(); host.dispose(); configureAppClock(undefined); throw error; });
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
  const launchTour = (id: string, trigger?: HTMLElement) => { tour?.stop('replaced'); tour = startTour(id, { router: app.config.globalProperties.$router, capabilities: discovery.methods.map((method) => method.name), locale: options.locale, trigger, taskState }); };
  tourActions.start = launchTour;
  toolbarState.ready = true;
  if (options.tour) launchTour(options.tour);
  const timer = setInterval(() => host.tick(), 2000);
  // Observable state has no mutators or test-only globals, and is useful for support diagnostics.
  const metricsTimer = setInterval(() => { toolbarState.paused = host.paused; toolbarState.metrics = JSON.stringify({ host: host.diagnostics(), client: client.diagnostics?.() }); }, 500);
  let disposed = false;
  function cleanup() { if (disposed) return; disposed = true; tour?.stop('disposed'); stopTour(); clearInterval(timer); clearInterval(metricsTimer); toolbarApp.unmount(); client.detach(); host.dispose(); configureAppClock(undefined); window.removeEventListener('pagehide', onPageHide); window.removeEventListener('pageshow', restore); }
  function dispose() { if (disposed) return; cleanup(); app.unmount(); }
  function restore(event: PageTransitionEvent) { if (event.persisted) location.reload(); }
  function onPageHide(event: PageTransitionEvent) { dispose(); if (event.persisted) window.addEventListener('pageshow', restore, { once: true }); }
  window.addEventListener('pagehide', onPageHide);
  app.onUnmount(cleanup);
  return { app, host, dispose };
}
