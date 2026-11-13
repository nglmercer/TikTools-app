export const scenarios = {
  connection: { route: '/connections', en: 'First connection', es: 'Primera conexión', pt: 'Primeira conexão', fr: 'Première connexion' },
  feed: { route: '/feed', en: 'LIVE feed', es: 'Feed LIVE', pt: 'Feed LIVE', fr: 'Flux LIVE' },
  automation: { route: '/automations', en: 'First automation', es: 'Primera automatización', pt: 'Primeira automação', fr: 'Première automatisation' },
  points: { route: '/points', en: 'Points', es: 'Puntos', pt: 'Pontos', fr: 'Points' },
  analytics: { route: '/analytics', en: 'Statistics', es: 'Estadísticas', pt: 'Estatísticas', fr: 'Statistiques' },
  overlays: { route: '/overlays', en: 'Overlays / OBS', es: 'Overlays / OBS', pt: 'Overlays / OBS', fr: 'Overlays / OBS' },
  plugins: { route: '/plugins', en: 'Plugins', es: 'Plugins', pt: 'Plugins', fr: 'Plugins' },
  cloud: { route: '/automations?view=variables', en: 'Cloud changes', es: 'Cambios Cloud', pt: 'Alterações Cloud', fr: 'Modifications Cloud' },
  collaboration: { route: '/settings?section=collaboration', en: 'Collaboration', es: 'Colaboración', pt: 'Colaboração', fr: 'Collaboration' },
  settings: { route: '/settings', en: 'Preferences', es: 'Preferencias', pt: 'Preferências', fr: 'Préférences' },
  diagnostics: { route: '/debug', en: 'Diagnostics / errors', es: 'Diagnósticos / errores', pt: 'Diagnósticos / erros', fr: 'Diagnostics / erreurs' },
} as const;
export type ScenarioId = keyof typeof scenarios;
export const demoLocales = ['en', 'es', 'pt', 'fr'] as const;
export type DemoLocale = typeof demoLocales[number];
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
  const scenario = params.get('scenario') ?? 'connection';
  const locale = params.get('locale') ?? 'es';
  const route = params.get('route') ?? (isScenario(scenario) ? scenarios[scenario].route : '');
  const instance = params.get('instance') ?? 'standalone';
  const tour = params.get('tour');
  if (!isScenario(scenario) || !(demoLocales as readonly string[]).includes(locale) || !validDemoRoute(route) || !/^[a-zA-Z0-9_-]{1,64}$/.test(instance) || (tour !== null && !['connection', 'automation', 'plugin', 'overlay', 'points', 'changes', 'diagnostics'].includes(tour))) throw new Error('Invalid demo parameters.');
  return { scenario, locale: locale as DemoLocale, route, instance, tour };
}
