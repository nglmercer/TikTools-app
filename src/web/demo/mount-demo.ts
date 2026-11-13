import { createControlClient } from '@/shared/api/control-client.ts';
import { mountWorkspace } from '@/app/mount-workspace.ts';
import { createDemoHost } from './runtime/host.ts';
import { scenarios, validDemoRoute, type parseDemoOptions, type DemoLocale } from './scenarios.ts';
import '@/app/styles/tokens.css';
import '@/app/styles/base.css';
import '@/app/styles/responsive.css';
import { startTour, stopTour, tourRegistry, type GuideSession } from '@/features/guided-help/index.ts';
import { configureAppClock } from '@/shared/lib/app-clock.ts';
import { DEMO_AT } from './fixtures/scenario-state.ts';
import { diagnosticLog, diagnosticView } from '@/shared/diagnostics/log.ts';
import { tourCatalogs } from '@/features/guided-help/catalogs/index.ts';

const labels = {
  en: { demo: 'Demo · fictional data', pause: 'Pause', play: 'Play', step: 'Next event', reset: 'Reset', error: 'Toggle failure', limit: 'Native effects, TikTok, OBS and real accounts are simulated.', catalog: 'Components', guide: 'Guide' },
  es: { demo: 'Demo · datos ficticios', pause: 'Pausar', play: 'Reproducir', step: 'Siguiente evento', reset: 'Reiniciar', error: 'Alternar fallo', limit: 'Los efectos nativos, TikTok, OBS y las cuentas reales se simulan.', catalog: 'Componentes', guide: 'Guía' },
  pt: { demo: 'Demo · dados fictícios', pause: 'Pausar', play: 'Reproduzir', step: 'Próximo evento', reset: 'Reiniciar', error: 'Alternar falha', limit: 'Efeitos nativos, TikTok, OBS e contas reais são simulados.', catalog: 'Componentes', guide: 'Guia' },
  fr: { demo: 'Démo · données fictives', pause: 'Pause', play: 'Lecture', step: 'Événement suivant', reset: 'Réinitialiser', error: 'Basculer la panne', limit: 'Les effets natifs, TikTok, OBS et les comptes réels sont simulés.', catalog: 'Composants', guide: 'Guide' },
} satisfies Record<DemoLocale, Record<string, string>>;

export async function bootDemo(options: ReturnType<typeof parseDemoOptions>, storage: { clear(): void }) {
  const host = createDemoHost(options.scenario);
  configureAppClock(() => DEMO_AT + host.diagnostics().ticks * 1000);
  const client = createControlClient({ transport: host.transport });
  if (!validDemoRoute(location.hash.slice(1))) location.hash = options.route;
  const t = labels[options.locale];
  const recordFailure = () => diagnosticLog.record({ source: 'transport', level: 'error', code: 'demo.simulated_disconnect', message: ({ en: 'Simulated connection interrupted. Retry or reset the demo.', es: 'Conexión simulada interrumpida. Reintenta o reinicia la demo.', pt: 'Conexão simulada interrompida. Tente novamente ou reinicie a demo.', fr: 'Connexion simulée interrompue. Réessayez ou réinitialisez la démo.' })[options.locale], requestId: 'demo-request' });
  if (options.scenario === 'diagnostics') recordFailure();
  const toolbar = document.querySelector('#demo-controls')!;
  const title = document.createElement('strong'); title.textContent = t.demo; toolbar.append(title);
  const select = document.createElement('select'); select.setAttribute('aria-label', t.demo);
  for (const [id, scenario] of Object.entries(scenarios)) { const option = document.createElement('option'); option.value = id; option.textContent = scenario[options.locale]; option.selected = id === options.scenario; select.append(option); }
  const selectLocale = document.createElement('select'); selectLocale.setAttribute('aria-label', 'Language / Idioma / Langue');
  for (const locale of ['en', 'es', 'pt', 'fr']) { const option = document.createElement('option'); option.value = locale; option.textContent = locale; option.selected = locale === options.locale; selectLocale.append(option); }
  const onScenario = () => { const url = new URL(location.href); url.searchParams.set('scenario', select.value); url.searchParams.delete('route'); url.searchParams.delete('tour'); url.hash = ''; location.assign(url.href); };
  const onLocale = () => { const url = new URL(location.href); url.searchParams.set('locale', selectLocale.value); location.assign(url.href); };
  select.addEventListener('change', onScenario); selectLocale.addEventListener('change', onLocale); toolbar.append(select, selectLocale);
  const removers: Array<() => void> = [];
  function button(text: string, testid: string, handler: (button: HTMLButtonElement) => void) { const element = document.createElement('button'); element.type = 'button'; element.textContent = text; element.dataset.testid = testid; const listener = () => handler(element); element.addEventListener('click', listener); removers.push(() => element.removeEventListener('click', listener)); toolbar.append(element); return element; }
  button(t.play, 'demo-play', (element) => { host.setPaused(!host.paused); element.textContent = host.paused ? t.play : t.pause; });
  button(t.step, 'demo-step', () => host.step());
  button(t.reset, 'demo-reset', () => { storage.clear(); location.reload(); });
  let failure = false;
  button(t.error, 'demo-error', (element) => { failure = !failure; host.simulateError(failure); if (failure) recordFailure(); element.setAttribute('aria-pressed', String(failure)); });
  const catalog = document.createElement('a'); catalog.textContent = t.catalog; catalog.href = '#/ui-kit'; toolbar.append(catalog);
  const note = document.createElement('small'); note.textContent = t.limit; toolbar.append(note);
  const app = await mountWorkspace({ client, locale: options.locale, configure(app) {
    const router = app.config.globalProperties.$router;
    router.beforeEach((to) => validDemoRoute(to.fullPath) ? true : '/feed');
  } });
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
  const tourSelect = document.createElement('select'); tourSelect.setAttribute('aria-label', t.guide);
  for (const id of Object.keys(tourRegistry) as Array<keyof typeof tourRegistry>) { const option = document.createElement('option'); option.value = id; option.textContent = Object.values(tourCatalogs[options.locale][id])[0]!.title; tourSelect.append(option); }
  toolbar.append(tourSelect);
  const launchTour = (id: string, trigger?: HTMLElement) => { tour?.stop('replaced'); tour = startTour(id, { router: app.config.globalProperties.$router, capabilities: discovery.methods.map((method) => method.name), locale: options.locale, trigger, taskState }); };
  button(t.guide, 'demo-guide', (element) => launchTour(tourSelect.value, element));
  if (options.tour) { tourSelect.value = options.tour; launchTour(options.tour); }
  const timer = setInterval(() => host.tick(), 2000);
  // Observable state has no mutators or test-only globals, and is useful for support diagnostics.
  const metrics = document.createElement('output'); metrics.dataset.testid = 'demo-metrics'; metrics.setAttribute('aria-live', 'off'); toolbar.append(metrics);
  const metricsTimer = setInterval(() => { metrics.value = JSON.stringify({ host: host.diagnostics(), client: client.diagnostics?.() }); }, 500);
  let disposed = false;
  function cleanup() { if (disposed) return; disposed = true; tour?.stop('disposed'); stopTour(); clearInterval(timer); clearInterval(metricsTimer); for (const remove of removers) remove(); select.removeEventListener('change', onScenario); selectLocale.removeEventListener('change', onLocale); client.detach(); host.dispose(); configureAppClock(undefined); window.removeEventListener('pagehide', onPageHide); window.removeEventListener('pageshow', restore); }
  function dispose() { if (disposed) return; cleanup(); app.unmount(); }
  function restore(event: PageTransitionEvent) { if (event.persisted) location.reload(); }
  function onPageHide(event: PageTransitionEvent) { dispose(); if (event.persisted) window.addEventListener('pageshow', restore, { once: true }); }
  window.addEventListener('pagehide', onPageHide);
  app.onUnmount(cleanup);
  return { app, host, dispose };
}
