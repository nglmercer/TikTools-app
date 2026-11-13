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
    return { text: title, link: `/${locale}/${audience}/${relative(root, path).replaceAll('\\', '/').replace(/index\.md$/, '').replace(/\.md$/, '.html')}`, order: Number(text.match(/^order: (\d+)$/m)?.[1] ?? 100) };
  });
  return pages.sort((a, b) => a.order - b.order || a.text.localeCompare(b.text)).map(({ text, link }) => ({ text, link }));
}
