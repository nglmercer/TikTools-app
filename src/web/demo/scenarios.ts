import { supportedLocales, normalizeLocale, type Locale } from '../shared/i18n/supported-locales.ts';
import { tourRegistry } from '@/features/guided-help/model/registry.ts';
import { demoLimits, demoParam, FLAG_ON } from './contract.ts';

/** One name per demo section; scenarios, embedded panels and the route guard all derive from it. */
export const demoRoute = {
  connections: '/connections',
  feed: '/feed',
  automations: '/automations',
  points: '/points',
  analytics: '/analytics',
  overlays: '/overlays',
  plugins: '/plugins',
  settings: '/settings',
  debug: '/debug',
  uiKit: '/ui-kit',
  components: '/components',
} as const;
/** The Cloud variables view is the automations route with a query. */
export const cloudRoute = `${demoRoute.automations}?view=variables` as const;
export const DEFAULT_DEMO_LOCALE = 'es';

export const scenarios = {
  connection: { route: demoRoute.connections, en: 'First connection', es: 'Primera conexión', pt: 'Primeira conexão', fr: 'Première connexion', zh: '首次连接', ja: '初めての接続' },
  feed: { route: demoRoute.feed, en: 'LIVE feed', es: 'Feed LIVE', pt: 'Feed LIVE', fr: 'Flux LIVE', zh: 'LIVE 动态', ja: 'LIVE フィード' },
  automation: { route: demoRoute.automations, en: 'First automation', es: 'Primera automatización', pt: 'Primeira automação', fr: 'Première automatisation', zh: '首条自动化规则', ja: '初めての自動化' },
  points: { route: demoRoute.points, en: 'Points', es: 'Puntos', pt: 'Pontos', fr: 'Points', zh: '积分', ja: 'ポイント' },
  analytics: { route: demoRoute.analytics, en: 'Statistics', es: 'Estadísticas', pt: 'Estatísticas', fr: 'Statistiques', zh: '统计', ja: '統計' },
  overlays: { route: demoRoute.overlays, en: 'Overlays / OBS', es: 'Overlays / OBS', pt: 'Overlays / OBS', fr: 'Overlays / OBS', zh: '叠加层 / OBS', ja: 'オーバーレイ / OBS' },
  plugins: { route: demoRoute.plugins, en: 'Plugins', es: 'Plugins', pt: 'Plugins', fr: 'Plugins', zh: '插件', ja: 'プラグイン' },
  cloud: { route: cloudRoute, en: 'Cloud changes', es: 'Cambios Cloud', pt: 'Alterações Cloud', fr: 'Modifications Cloud', zh: 'Cloud 更改', ja: 'Cloud の変更' },
  settings: { route: demoRoute.settings, en: 'Preferences', es: 'Preferencias', pt: 'Preferências', fr: 'Préférences', zh: '偏好设置', ja: '環境設定' },
  diagnostics: { route: demoRoute.debug, en: 'Diagnostics / errors', es: 'Diagnósticos / errores', pt: 'Diagnósticos / erros', fr: 'Diagnostics / erreurs', zh: '诊断与错误', ja: '診断とエラー' },
} as const;
export type ScenarioId = keyof typeof scenarios;
export const demoLocales = supportedLocales;
export type DemoLocale = Locale;
export const embeddedPanels = { connection: demoRoute.connections, feed: demoRoute.feed, automation: demoRoute.automations, points: demoRoute.points, overlays: demoRoute.overlays } as const;
export type EmbeddedPanel = keyof typeof embeddedPanels;
export function isScenario(value: string): value is ScenarioId { return Object.hasOwn(scenarios, value); }
export const demoRoutes = Object.values(demoRoute);
export function validDemoRoute(value: string): boolean {
  if (!value.startsWith('/') || value.startsWith('//') || /[\\#]/.test(value) || [...value].some((character) => character.charCodeAt(0) < 32)) return false;
  const url = new URL(value, 'https://demo.invalid');
  if (!demoRoutes.some((route) => url.pathname === route || (route === demoRoute.components && /^\/components\/[a-z-]+$/.test(url.pathname)))) return false;
  if ([...url.searchParams].length > 8) return false;
  return [...url.searchParams].every(([key, value]) => {
    if (['tab', 'section', 'view'].includes(key)) return /^[a-z-]{1,32}$/.test(value);
    if (url.pathname !== demoRoute.debug) return false;
    if (key === 'returnTo') return value.length <= 512 && !value.startsWith(demoRoute.debug) && validDemoRoute(value);
    return ['source', 'entity', 'entry'].includes(key) && /^[a-zA-Z0-9_.:-]{0,160}$/.test(value);
  });
}
/** Single digits, then 10 through maxInitialEvents. Leading zeros are rejected. */
const initialEventCount = new RegExp(`^(?:[0-9]|1[0-${demoLimits.maxInitialEvents - 10}])$`);
export function parseDemoOptions(search: string) {
  const params = new URLSearchParams(search);
  if (params.has(demoParam.framed) && params.get(demoParam.framed) !== FLAG_ON) throw new Error('Invalid frame presentation.');
  const framed = params.get(demoParam.framed) === FLAG_ON;
  if (params.has(demoParam.preview) && params.get(demoParam.preview) !== FLAG_ON) throw new Error('Invalid preview presentation.');
  const preview = params.get(demoParam.preview) === FLAG_ON;
  if (params.has(demoParam.showcase) && params.get(demoParam.showcase) !== FLAG_ON) throw new Error('Invalid showcase presentation.');
  const showcase = params.get(demoParam.showcase) === FLAG_ON;
  if (params.has(demoParam.record) && (params.get(demoParam.record) !== FLAG_ON || !showcase)) throw new Error('Recording requires the showcase.');
  const record = params.get(demoParam.record) === FLAG_ON;
  const scenario = params.get(demoParam.scenario) ?? (showcase ? 'feed' : 'connection');
  const locale = normalizeLocale(params.get(demoParam.locale) ?? DEFAULT_DEMO_LOCALE);
  const embed = params.get(demoParam.embed);
  if (embed !== null && !Object.hasOwn(embeddedPanels, embed)) throw new Error('Invalid embedded panel.');
  const panel = embed as EmbeddedPanel | null;
  const events = params.get(demoParam.events);
  if (events !== null && (panel !== 'feed' || !initialEventCount.test(events))) throw new Error('Invalid initial event count.');
  const initialEvents = events === null ? panel === 'feed' ? 1 : 0 : Number(events);
  const route = params.get(demoParam.route) ?? (panel ? embeddedPanels[panel] : isScenario(scenario) ? scenarios[scenario].route : '');
  const instance = params.get(demoParam.instance) ?? 'standalone';
  const tour = params.get(demoParam.tour);
  if (!isScenario(scenario) || !locale || !validDemoRoute(route) || !/^[a-zA-Z0-9_-]{1,64}$/.test(instance) || (tour !== null && !Object.hasOwn(tourRegistry, tour))) throw new Error('Invalid demo parameters.');
  if (panel && (route !== embeddedPanels[panel] || tour)) throw new Error('Embedded panels require their focused route without a tour.');
  if (showcase && (panel || tour || preview || scenario !== 'feed')) throw new Error('The showcase runs on the feed scenario without a tour, panel or preview.');
  return { scenario, locale, route, instance, tour, embed: panel, initialEvents, framed, preview, showcase, record };
}
