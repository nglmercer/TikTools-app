import { resolve } from 'node:path';
import { defineConfig } from 'vitepress';
import { siteConfig } from '../../scripts/lib/site-config.ts';
import { audienceSidebar } from './navigation.ts';
import { writeFile } from 'node:fs/promises';
import { buildProvenance } from '../../scripts/lib/build-provenance.ts';
import { renderDocSearch } from '../scripts/search-policy.ts';
import { tokenizeDocSearch } from '../scripts/search-tokenizer.ts';
import { docLocales, docCopy, localizedDocumentPath, type DocLocale } from '../locales.ts';

const site = siteConfig();
const content = resolve(import.meta.dirname, '../content');

function theme(locale: DocLocale) {
  const t = docCopy[locale];
  return {
    nav: [{ text: t.audience, items: [{ text: t.user, link: `/${locale}/user/` }, { text: t.developer, link: `/${locale}/developer/` }] }, { text: t.releases, link: `/${locale}/releases/` }, { text: t.demo, link: `/demo/?locale=${locale}`, target: '_self' }],
    sidebar: { [`/${locale}/user/`]: audienceSidebar(content, locale, 'user'), [`/${locale}/developer/`]: audienceSidebar(content, locale, 'developer') },
    editLink: { pattern: 'https://github.com/nglmercer/TikTools-app/edit/remake/docs/content/:path', text: t.edit },
    docFooter: { prev: t.previous, next: t.next }, outline: { label: t.outline, level: [2, 3] as [number, number] },
    sidebarMenuLabel: t.navigation, returnToTopLabel: t.top, langMenuLabel: t.language,
    darkModeSwitchLabel: t.theme, darkModeSwitchTitle: t.darkTitle, lightModeSwitchTitle: t.lightTitle, skipToContentLabel: t.skip,
    lastUpdated: { text: t.lastUpdated, formatOptions: { dateStyle: 'medium' as const, timeStyle: 'short' as const, forceLocale: true } },
  };
}

const searchLocales = Object.fromEntries(docLocales.map((locale) => {
  const t = docCopy[locale];
  return [locale, { translations: {
    button: { buttonText: t.search, buttonAriaLabel: t.searchLabel },
    modal: { displayDetails: t.details, resetButtonTitle: t.reset, backButtonTitle: t.closeSearch, noResultsText: t.noResults,
      footer: { selectText: t.select, selectKeyAriaLabel: t.enter, navigateText: t.navigate, navigateUpKeyAriaLabel: t.up, navigateDownKeyAriaLabel: t.down, closeText: t.close, closeKeyAriaLabel: t.escape } },
  } }];
}));

export default defineConfig({
  title: 'TikTools', description: 'TikTools user guides, developer documentation and an interactive Vue demo.',
  srcDir: './content', outDir: '../dist/docs', cacheDir: '.vitepress/cache', base: site.base, cleanUrls: false,
  lastUpdated: true, ignoreDeadLinks: false,
  locales: Object.fromEntries(docLocales.map((locale) => [locale, { label: docCopy[locale].label, lang: locale, link: `/${locale}/`, description: docCopy[locale].description, themeConfig: theme(locale) }])),
  themeConfig: {
    siteTitle: 'TikTools', ...theme('en'),
    socialLinks: [{ icon: 'github', link: 'https://github.com/nglmercer/TikTools-app' }],
    search: { provider: 'local', options: { _render: renderDocSearch, miniSearch: { options: { tokenize: tokenizeDocSearch } }, locales: searchLocales } },
  },
  sitemap: { hostname: site.url },
  async buildEnd(config) { await writeFile(resolve(config.outDir, '.build-info.json'), JSON.stringify(buildProvenance('docs')) + '\n'); },
  transformHead({ pageData }) {
    const path = pageData.relativePath.replace(/index\.md$/, '').replace(/\.md$/, '.html');
    const url = new URL(path, site.url).href;
    const tail = localizedDocumentPath(path)?.tail;
    return [['link', { rel: 'canonical', href: url }], ...docLocales.map((lang): ['link', Record<string, string>] => ['link', { rel: 'alternate', hreflang: lang, href: new URL(tail === undefined ? `${lang}/` : `${lang}/${tail}`, site.url).href }]), ['link', { rel: 'alternate', hreflang: 'x-default', href: new URL(tail === undefined ? 'en/' : `en/${tail}`, site.url).href }]];
  },
  vite: { resolve: { dedupe: ['vue'], alias: { '@': resolve(import.meta.dirname, '../../src/web') } } },
});
