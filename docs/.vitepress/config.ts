import { resolve } from 'node:path';
import { defineConfig } from 'vitepress';
import { siteConfig } from '../../scripts/lib/site-config.ts';
import { audienceSidebar } from './navigation.ts';
import { writeFile } from 'node:fs/promises';
import { buildProvenance } from '../../scripts/lib/build-provenance.ts';
import { renderDocSearch } from '../scripts/search-policy.ts';

const site = siteConfig();
const content = resolve(import.meta.dirname, '../content');

function theme(locale: 'en' | 'es') {
  const es = locale === 'es';
  return {
    nav: [{ text: es ? 'Audiencia' : 'Audience', items: [{ text: es ? 'Usuario' : 'User', link: `/${locale}/user/` }, { text: es ? 'Desarrollo' : 'Developer', link: `/${locale}/developer/` }] }, { text: es ? 'Versiones' : 'Releases', link: `/${locale}/releases/` }, { text: 'Demo', link: '/demo/', target: '_self' }],
    sidebar: { [`/${locale}/user/`]: audienceSidebar(content, locale, 'user'), [`/${locale}/developer/`]: audienceSidebar(content, locale, 'developer') },
    editLink: { pattern: 'https://github.com/nglmercer/TikTools-app/edit/remake/docs/content/:path', text: es ? 'Editar esta página' : 'Edit this page' },
    docFooter: { prev: es ? 'Anterior' : 'Previous', next: es ? 'Siguiente' : 'Next' },
    outline: { label: es ? 'En esta página' : 'On this page', level: [2, 3] as [number, number] },
    sidebarMenuLabel: es ? 'Navegación' : 'Navigation', returnToTopLabel: es ? 'Volver arriba' : 'Return to top',
    langMenuLabel: es ? 'Cambiar idioma' : 'Change language', darkModeSwitchLabel: es ? 'Tema' : 'Theme',
    skipToContentLabel: es ? 'Saltar al contenido' : 'Skip to content',
  };
}

export default defineConfig({
  title: 'TikTools', description: 'TikTools user guides, developer documentation and an interactive Vue demo.',
  srcDir: './content', outDir: '../dist/docs', cacheDir: '.vitepress/cache', base: site.base, cleanUrls: false,
  lastUpdated: true, ignoreDeadLinks: false,
  locales: { en: { label: 'English', lang: 'en', link: '/en/', themeConfig: theme('en') }, es: { label: 'Español', lang: 'es', link: '/es/', description: 'Guías de usuario, documentación técnica y demo interactiva de TikTools.', themeConfig: theme('es') } },
  themeConfig: {
    siteTitle: 'TikTools', ...theme('en'),
    socialLinks: [{ icon: 'github', link: 'https://github.com/nglmercer/TikTools-app' }],
    search: { provider: 'local', options: { _render: renderDocSearch, locales: { es: { translations: { button: { buttonText: 'Buscar', buttonAriaLabel: 'Buscar documentación' }, modal: { displayDetails: 'Mostrar detalles', resetButtonTitle: 'Borrar búsqueda', backButtonTitle: 'Cerrar búsqueda', noResultsText: 'Sin resultados', footer: { selectText: 'seleccionar', selectKeyAriaLabel: 'intro', navigateText: 'navegar', navigateUpKeyAriaLabel: 'flecha arriba', navigateDownKeyAriaLabel: 'flecha abajo', closeText: 'cerrar', closeKeyAriaLabel: 'escape' } } } } } } },
  },
  sitemap: { hostname: site.url },
  async buildEnd(config) { await writeFile(resolve(config.outDir, '.build-info.json'), JSON.stringify(buildProvenance('docs')) + '\n'); },
  transformHead({ pageData }) {
    const path = pageData.relativePath.replace(/index\.md$/, '').replace(/\.md$/, '.html');
    const url = new URL(path, site.url).href;
    const tail = path.match(/^(en|es)\/(.*)$/)?.[2];
    return [['link', { rel: 'canonical', href: url }], ...(['en', 'es'] as const).map((lang): ['link', Record<string, string>] => ['link', { rel: 'alternate', hreflang: lang, href: new URL(tail === undefined ? `${lang}/` : `${lang}/${tail}`, site.url).href }]), ['link', { rel: 'alternate', hreflang: 'x-default', href: new URL(tail === undefined ? 'en/' : `en/${tail}`, site.url).href }]];
  },
  vite: { resolve: { dedupe: ['vue'], alias: { '@': resolve(import.meta.dirname, '../../src/web') } } },
});
