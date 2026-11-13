import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import type { DefaultTheme } from 'vitepress';

function markdown(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => entry.isDirectory() ? markdown(join(directory, entry.name)) : entry.name.endsWith('.md') ? [join(directory, entry.name)] : []);
}

/** Navigation is derived only from existing reviewed articles, never from planned pages. */
export function audienceSidebar(content: string, locale: string, audience: string): DefaultTheme.SidebarItem[] {
  const root = join(content, locale, audience);
  const pages = markdown(root).map((path) => {
    const text = readFileSync(path, 'utf8');
    const title = text.match(/^title: (.+)$/m)?.[1]?.replace(/^"|"$/g, '') ?? relative(root, path);
    const localPath = relative(root, path).replaceAll('\\', '/');
    return { text: title, path: localPath, link: `/${locale}/${audience}/${localPath.replace(/index\.md$/, '').replace(/\.md$/, '.html')}`, order: Number(text.match(/^order: (\d+)$/m)?.[1] ?? 100) };
  });
  const es = locale === 'es';
  const groups: Array<{ en: string; es: string; accepts(path: string): boolean }> = audience === 'user' ? [
    { en: 'Getting started', es: 'Empezar', accepts: (path) => !path.includes('/') },
    ...[['live', 'LIVE', 'LIVE'], ['points', 'Points and statistics', 'Puntos y estadísticas'], ['automations', 'Automations', 'Automatizaciones'], ['plugins', 'Plugins and audio', 'Plugins y audio'], ['overlays', 'OBS and overlays', 'OBS y overlays'], ['cloud', 'Cloud and collaboration', 'Cloud y colaboración'], ['preferences', 'Preferences and support', 'Preferencias y soporte']].map(([folder, en, es]) => ({ en: en!, es: es!, accepts: (path: string) => path.startsWith(`${folder}/`) })),
  ] : [
    { en: 'Development setup', es: 'Preparación', accepts: (path) => ['index.md', 'setup.md', 'platforms.md'].includes(path) },
    { en: 'Architecture', es: 'Arquitectura', accepts: (path) => ['architecture.md', 'frontend.md', 'lifecycle.md', 'storage.md', 'events.md', 'performance.md', 'transports.md'].includes(path) },
    { en: 'API and collaboration', es: 'API y colaboración', accepts: (path) => ['control-api.md', 'agents.md', 'remote-client.md', 'collaboration.md', 'cloud.md'].includes(path) },
    { en: 'Automations', es: 'Automatizaciones', accepts: (path) => ['automations.md', 'templates.md'].includes(path) },
    { en: 'Plugins', es: 'Plugins', accepts: (path) => path.startsWith('plugins/') },
    { en: 'Overlays', es: 'Overlays', accepts: (path) => path === 'overlays.md' },
    { en: 'Components', es: 'Componentes', accepts: (path) => path.startsWith('components/') },
    { en: 'Quality and maintenance', es: 'Calidad y mantenimiento', accepts: () => true },
  ];
  const remaining = new Set(pages);
  return groups.flatMap((group, index) => {
    const items = [...remaining].filter((page) => group.accepts(page.path)).sort((a, b) => a.order - b.order || a.text.localeCompare(b.text));
    for (const item of items) remaining.delete(item);
    return items.length ? [{ text: es ? group.es : group.en, collapsed: index !== 0, items: items.map(({ text, link }) => ({ text, link })) }] : [];
  });
}
