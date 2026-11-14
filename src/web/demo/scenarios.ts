import { supportedLocales, normalizeLocale, type Locale } from '../shared/i18n/supported-locales.ts';

export const scenarios = {
  connection: { route: '/connections', en: 'First connection', es: 'Primera conexión', pt: 'Primeira conexão', fr: 'Première connexion', zh: '首次连接', ja: '初めての接続' },
  feed: { route: '/feed', en: 'LIVE feed', es: 'Feed LIVE', pt: 'Feed LIVE', fr: 'Flux LIVE', zh: 'LIVE 动态', ja: 'LIVE フィード' },
  automation: { route: '/automations', en: 'First automation', es: 'Primera automatización', pt: 'Primeira automação', fr: 'Première automatisation', zh: '首条自动化规则', ja: '初めての自動化' },
  points: { route: '/points', en: 'Points', es: 'Puntos', pt: 'Pontos', fr: 'Points', zh: '积分', ja: 'ポイント' },
  analytics: { route: '/analytics', en: 'Statistics', es: 'Estadísticas', pt: 'Estatísticas', fr: 'Statistiques', zh: '统计', ja: '統計' },
  overlays: { route: '/overlays', en: 'Overlays / OBS', es: 'Overlays / OBS', pt: 'Overlays / OBS', fr: 'Overlays / OBS', zh: '叠加层 / OBS', ja: 'オーバーレイ / OBS' },
  plugins: { route: '/plugins', en: 'Plugins', es: 'Plugins', pt: 'Plugins', fr: 'Plugins', zh: '插件', ja: 'プラグイン' },
  cloud: { route: '/automations?view=variables', en: 'Cloud changes', es: 'Cambios Cloud', pt: 'Alterações Cloud', fr: 'Modifications Cloud', zh: 'Cloud 更改', ja: 'Cloud の変更' },
  collaboration: { route: '/settings?section=collaboration', en: 'Collaboration', es: 'Colaboración', pt: 'Colaboração', fr: 'Collaboration', zh: '协作', ja: '共同作業' },
  settings: { route: '/settings', en: 'Preferences', es: 'Preferencias', pt: 'Preferências', fr: 'Préférences', zh: '偏好设置', ja: '環境設定' },
  diagnostics: { route: '/debug', en: 'Diagnostics / errors', es: 'Diagnósticos / errores', pt: 'Diagnósticos / erros', fr: 'Diagnostics / erreurs', zh: '诊断与错误', ja: '診断とエラー' },
} as const;
export type ScenarioId = keyof typeof scenarios;
export const demoLocales = supportedLocales;
export type DemoLocale = Locale;
export const embeddedPanels = { connection: '/connections', feed: '/feed', automation: '/automations', points: '/points', overlays: '/overlays' } as const;
export type EmbeddedPanel = keyof typeof embeddedPanels;
export function isScenario(value: string): value is ScenarioId { return Object.hasOwn(scenarios, value); }
export const demoRoutes = ['/connections', '/feed', '/automations', '/points', '/analytics', '/overlays', '/plugins', '/settings', '/debug', '/ui-kit', '/components'] as const;
export function validDemoRoute(value: string): boolean {
  if (!value.startsWith('/') || value.startsWith('//') || /[\\#]/.test(value) || [...value].some((character) => character.charCodeAt(0) < 32)) return false;
  const url = new URL(value, 'https://demo.invalid');
  if (!demoRoutes.some((route) => url.pathname === route || (route === '/components' && /^\/components\/[a-z-]+$/.test(url.pathname)))) return false;
  if ([...url.searchParams].length > 8) return false;
  return [...url.searchParams].every(([key, value]) => {
    if (['tab', 'section', 'view'].includes(key)) return /^[a-z-]{1,32}$/.test(value);
    if (url.pathname !== '/debug') return false;
    if (key === 'returnTo') return value.length <= 512 && !value.startsWith('/debug') && validDemoRoute(value);
    return ['source', 'entity', 'entry'].includes(key) && /^[a-zA-Z0-9_.:-]{0,160}$/.test(value);
  });
}
export function parseDemoOptions(search: string) {
  const params = new URLSearchParams(search);
  if (params.has('framed') && params.get('framed') !== '1') throw new Error('Invalid frame presentation.');
  const framed = params.get('framed') === '1';
  const scenario = params.get('scenario') ?? 'connection';
  const locale = normalizeLocale(params.get('locale') ?? 'es');
  const embed = params.get('embed');
  if (embed !== null && !Object.hasOwn(embeddedPanels, embed)) throw new Error('Invalid embedded panel.');
  const panel = embed as EmbeddedPanel | null;
  const events = params.get('events');
  if (events !== null && (panel !== 'feed' || !/^(?:[0-9]|1[0-2])$/.test(events))) throw new Error('Invalid initial event count.');
  const initialEvents = events === null ? panel === 'feed' ? 1 : 0 : Number(events);
  const route = params.get('route') ?? (panel ? embeddedPanels[panel] : isScenario(scenario) ? scenarios[scenario].route : '');
  const instance = params.get('instance') ?? 'standalone';
  const tour = params.get('tour');
  if (!isScenario(scenario) || !locale || !validDemoRoute(route) || !/^[a-zA-Z0-9_-]{1,64}$/.test(instance) || (tour !== null && !['connection', 'automation', 'plugin', 'overlay', 'points', 'changes', 'diagnostics'].includes(tour))) throw new Error('Invalid demo parameters.');
  if (panel && (route !== embeddedPanels[panel] || tour)) throw new Error('Embedded panels require their focused route without a tour.');
  return { scenario, locale, route, instance, tour, embed: panel, initialEvents, framed };
}
