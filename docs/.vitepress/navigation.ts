import { readdirSync, readFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import type { DefaultTheme } from 'vitepress';
import { docCopy, type DocLocale } from '../locales.ts';

function markdown(directory: string): string[] {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => entry.isDirectory() ? markdown(join(directory, entry.name)) : entry.name.endsWith('.md') ? [join(directory, entry.name)] : []);
}

/** Navigation is derived only from existing reviewed articles, never from planned pages. */
export function audienceSidebar(content: string, locale: DocLocale, audience: string): DefaultTheme.SidebarItem[] {
  const root = join(content, locale, audience);
  const pages = markdown(root).map((path) => {
    const text = readFileSync(path, 'utf8');
    const title = text.match(/^title: (.+)$/m)?.[1]?.replace(/^"|"$/g, '') ?? relative(root, path);
    const localPath = relative(root, path).replaceAll('\\', '/');
    return { text: title, path: localPath, link: `/${locale}/${audience}/${localPath.replace(/index\.md$/, '').replace(/\.md$/, '.html')}`, order: Number(text.match(/^order: (\d+)$/m)?.[1] ?? 100) };
  });
  const labels = docCopy[locale].groups;
  const groups: Array<{ label: string; accepts(path: string): boolean }> = audience === 'user' ? [
    { label: labels[0], accepts: (path) => !path.includes('/') },
    ...['live', 'points', 'automations', 'plugins', 'overlays', 'cloud', 'preferences'].map((folder, i) => ({ label: labels[i + 1]!, accepts: (path: string) => path.startsWith(`${folder}/`) })),
  ] : [
    { label: labels[8], accepts: (path) => ['index.md', 'setup.md', 'platforms.md'].includes(path) },
    { label: labels[9], accepts: (path) => ['architecture.md', 'frontend.md', 'lifecycle.md', 'storage.md', 'events.md', 'performance.md', 'transports.md'].includes(path) },
    { label: labels[10], accepts: (path) => ['control-api.md', 'agents.md', 'remote-client.md', 'collaboration.md', 'cloud.md'].includes(path) },
    { label: labels[3], accepts: (path) => ['automations.md', 'templates.md'].includes(path) },
    { label: labels[11], accepts: (path) => path.startsWith('plugins/') },
    { label: labels[12], accepts: (path) => path === 'overlays.md' },
    { label: labels[13], accepts: (path) => path.startsWith('components/') },
    { label: labels[14], accepts: () => true },
  ];
  const remaining = new Set(pages);
  return groups.flatMap((group, index) => {
    const items = [...remaining].filter((page) => group.accepts(page.path)).sort((a, b) => a.order - b.order || a.text.localeCompare(b.text));
    for (const item of items) remaining.delete(item);
    return items.length ? [{ text: group.label, collapsed: index !== 0, items: items.map(({ text, link }) => ({ text, link })) }] : [];
  });
}
