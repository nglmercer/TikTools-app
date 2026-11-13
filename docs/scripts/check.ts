import { access } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { contentRoot, metadataErrors, readArticles } from './content.ts';
import { migrationErrors } from './migration.ts';
import { parseDemoOptions } from '../../src/web/demo/scenarios.ts';

const args = process.argv.slice(2);
if (args.some((arg) => arg !== '--content-only')) throw new Error('usage: bun docs/scripts/check.ts [--content-only]');
const articles = await readArticles();
const errors = metadataErrors(articles);
const byPath = new Map(articles.map((article) => [article.path, article]));
let internalLinks = 0;
let externalLinks = 0;
for (const article of articles) {
  for (const frame of article.source.matchAll(/<DemoFrame\s+([^>]+)\/?\s*>/g)) {
    const attributes = new Map([...frame[1]!.matchAll(/([a-z]+)="([^"]*)"/g)].map((match) => [match[1]!, match[2]!]));
    if (!attributes.get('title')) errors.push(`${article.relativePath}: demo iframe needs an accessible title`);
    try { parseDemoOptions(new URLSearchParams([...attributes].filter(([key]) => key !== 'title')).toString()); } catch { errors.push(`${article.relativePath}: invalid DemoFrame parameters`); }
  }
  for (const link of article.links) {
    if (/^(https?:|mailto:)/.test(link.target)) { externalLinks += 1; continue; }
    if (link.target.startsWith('/demo/')) { try { parseDemoOptions(new URL(link.target, 'https://site.invalid').search); } catch { errors.push(`${article.relativePath}: invalid demo URL`); } continue; }
    let pathname: string;
    let anchor: string;
    try {
      const [raw = '', hash = ''] = link.target.split('#', 2);
      pathname = decodeURIComponent(raw.split('?')[0] ?? '');
      anchor = decodeURIComponent(hash);
    } catch { errors.push(`${article.relativePath}: invalid link encoding ${link.target}`); continue; }
    let target = pathname ? resolve(pathname.startsWith('/') ? contentRoot : dirname(article.path), pathname.replace(/^\//, '')) : article.path;
    if (pathname.endsWith('/')) target = resolve(target, 'index.md');
    else if (target.endsWith('.html')) target = target.slice(0, -5) + '.md';
    if (link.image && pathname.startsWith('/')) target = resolve(contentRoot, 'public', pathname.slice(1));
    try { await access(target); } catch { errors.push(`${article.relativePath}: missing link/image ${link.target}`); continue; }
    if (anchor && byPath.has(target)) {
      const destination = byPath.get(target)!;
      const explicit = [...destination.source.matchAll(/<(?:a|span)[^>]*\bid=["']([^"']+)["']/g)].map((match) => match[1]);
      if (!destination.headings.some((heading) => heading.id === anchor) && !explicit.includes(anchor)) errors.push(`${article.relativePath}: missing anchor ${link.target}`);
    }
    internalLinks += 1;
  }
}
if (!args.includes('--content-only')) {
  errors.push(...await migrationErrors(resolve(import.meta.dirname, '../..')));
}
if (errors.length) throw new Error(`Documentation validation failed:\n${errors.map((error) => `- ${error}`).join('\n')}`);
console.log(`Checked ${articles.length} public articles, ${internalLinks} internal links/images and inventoried ${externalLinks} external links.`);
